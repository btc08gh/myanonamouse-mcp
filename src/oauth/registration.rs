// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use super::middleware::extract_client_ip;
use super::state::OAuthState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    redirect_uris: Vec<String>,
    client_name: Option<String>,
    // Accept but ignore these per spec
    #[allow(dead_code)]
    grant_types: Option<Vec<String>>,
    #[allow(dead_code)]
    response_types: Option<Vec<String>>,
    #[allow(dead_code)]
    token_endpoint_auth_method: Option<String>,
}

/// Validate that a redirect URI meets OAuth 2.1 requirements:
/// - Must be https:// (except localhost/127.0.0.1/[::1] which may use http://)
/// - Must not contain a fragment (#)
fn validate_redirect_uri(uri: &str) -> Result<(), String> {
    // No fragments allowed
    if uri.contains('#') {
        return Err(format!("redirect URI must not contain a fragment: {uri}"));
    }

    // Parse the URI to check the scheme and host
    let Ok(parsed) = url::Url::parse(uri) else {
        return Err(format!("invalid redirect URI: {uri}"));
    };

    let scheme = parsed.scheme();

    if scheme == "https" {
        return Ok(());
    }

    if scheme == "http" {
        let is_loopback = match parsed.host() {
            Some(url::Host::Domain(d)) => d == "localhost",
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        };
        if is_loopback {
            return Ok(());
        }
        return Err(format!(
            "redirect URI must use https:// (http:// only allowed for localhost): {uri}"
        ));
    }

    Err(format!("redirect URI must use https:// scheme: {uri}"))
}

/// `POST /register` (RFC 7591) — Dynamic client registration.
pub async fn handle_register(
    State(state): State<Arc<OAuthState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let client_ip = extract_client_ip(&headers);
    tracing::debug!(client_ip, "client registration request");

    if req.redirect_uris.is_empty() {
        warn!(client_ip, "registration rejected: no redirect URIs");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_client_metadata",
                "error_description": "at least one redirect_uri is required"
            })),
        );
    }

    for uri in &req.redirect_uris {
        if let Err(msg) = validate_redirect_uri(uri) {
            warn!(client_ip, uri, "registration rejected: invalid redirect URI");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_client_metadata",
                    "error_description": msg
                })),
            );
        }
    }

    match state.register_client(req.redirect_uris.clone(), req.client_name.clone()) {
        Ok(client_id) => {
            tracing::debug!(client_ip, client_id, "client registered");
            (
                StatusCode::CREATED,
                Json(json!({
                    "client_id": client_id,
                    "redirect_uris": req.redirect_uris,
                    "client_name": req.client_name,
                    "grant_types": ["authorization_code", "refresh_token"],
                    "response_types": ["code"],
                    "token_endpoint_auth_method": "none"
                })),
            )
        }
        Err(msg) => {
            warn!(client_ip, "registration rejected: {msg}");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_client_metadata",
                    "error_description": msg
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_redirect_uri_https() {
        assert!(validate_redirect_uri("https://example.com/callback").is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_http_localhost() {
        assert!(validate_redirect_uri("http://localhost:3000/callback").is_ok());
        assert!(validate_redirect_uri("http://127.0.0.1:8080/cb").is_ok());
        assert!(validate_redirect_uri("http://[::1]:8080/cb").is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_http_non_localhost() {
        assert!(validate_redirect_uri("http://example.com/callback").is_err());
    }

    #[test]
    fn test_validate_redirect_uri_fragment() {
        assert!(validate_redirect_uri("https://example.com/callback#frag").is_err());
    }

    #[test]
    fn test_validate_redirect_uri_invalid() {
        assert!(validate_redirect_uri("not a url").is_err());
    }
}
