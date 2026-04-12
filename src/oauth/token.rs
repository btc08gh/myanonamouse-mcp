// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, PRAGMA};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

use super::middleware::extract_client_ip;
use super::state::{OAuthState, ACCESS_TOKEN_LIFETIME_SECS};

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: Option<String>,
    // authorization_code fields
    code: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    // refresh_token fields
    refresh_token: Option<String>,
    // Accept and ignore
    #[allow(dead_code)]
    resource: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn token_error(error: &str, description: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        [
            (CACHE_CONTROL, "no-store"),
            (PRAGMA, "no-cache"),
        ],
        axum::Json(json!({
            "error": error,
            "error_description": description
        })),
    )
        .into_response()
}

fn token_success(access_token: &str, refresh_token: &str) -> Response {
    (
        StatusCode::OK,
        [
            (CACHE_CONTROL, "no-store"),
            (PRAGMA, "no-cache"),
        ],
        axum::Json(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": ACCESS_TOKEN_LIFETIME_SECS,
            "refresh_token": refresh_token,
            "scope": ""
        })),
    )
        .into_response()
}

/// Verify PKCE S256: base64url_no_pad(SHA-256(code_verifier)) == code_challenge
fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    let hash = Sha256::digest(code_verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(hash);
    // Use constant-time comparison to avoid timing attacks
    use subtle::ConstantTimeEq;
    let result: bool = computed.as_bytes().ct_eq(code_challenge.as_bytes()).into();
    result
}

// ---------------------------------------------------------------------------
// POST /token
// ---------------------------------------------------------------------------

pub async fn handle_token(
    State(state): State<Arc<OAuthState>>,
    headers: axum::http::HeaderMap,
    Form(req): Form<TokenRequest>,
) -> Response {
    let client_ip = extract_client_ip(&headers);

    match req.grant_type.as_deref() {
        Some("authorization_code") => handle_authorization_code(&state, &client_ip, req).await,
        Some("refresh_token") => handle_refresh_token(&state, &client_ip, req).await,
        Some(gt) => {
            warn!(client_ip, grant_type = gt, "token: unsupported grant_type");
            token_error("unsupported_grant_type", &format!("unsupported grant_type: {gt}"))
        }
        None => token_error("invalid_request", "missing grant_type"),
    }
}

async fn handle_authorization_code(
    state: &OAuthState,
    client_ip: &str,
    req: TokenRequest,
) -> Response {
    let Some(code) = &req.code else {
        return token_error("invalid_request", "missing code");
    };
    let Some(client_id) = &req.client_id else {
        return token_error("invalid_request", "missing client_id");
    };
    let Some(redirect_uri) = &req.redirect_uri else {
        return token_error("invalid_request", "missing redirect_uri");
    };
    let Some(code_verifier) = &req.code_verifier else {
        return token_error("invalid_request", "missing code_verifier");
    };

    // Verify client still exists
    if !state.client_exists(client_id) {
        warn!(client_ip, client_id, "token: client_id not found");
        return token_error("invalid_client", "unknown client_id");
    }

    // Consume the authorization code (single-use)
    let Some(auth_code) = state.take_auth_code(code) else {
        warn!(client_ip, client_id, "token: invalid or expired authorization code");
        return token_error("invalid_grant", "invalid or expired authorization code");
    };

    // Verify the code was issued to this client
    if auth_code.client_id != *client_id {
        warn!(client_ip, client_id, "token: client_id mismatch on code exchange");
        return token_error("invalid_grant", "authorization code was not issued to this client");
    }

    // Verify redirect_uri matches
    if auth_code.redirect_uri != *redirect_uri {
        warn!(client_ip, client_id, "token: redirect_uri mismatch on code exchange");
        return token_error("invalid_grant", "redirect_uri does not match");
    }

    // Verify PKCE
    if !verify_pkce(code_verifier, &auth_code.code_challenge) {
        warn!(client_ip, client_id, "token: PKCE verification failed");
        return token_error("invalid_grant", "PKCE verification failed");
    }

    // Mark client as authorized (prevents 15-min expiry cleanup)
    state.mark_client_authorized(client_id);

    // Issue tokens
    let access_token = state.insert_access_token(client_id.clone());
    let refresh_token = state.insert_refresh_token(client_id.clone());

    debug!(client_ip, client_id, "access token issued via authorization_code");
    token_success(&access_token, &refresh_token)
}

async fn handle_refresh_token(
    state: &OAuthState,
    client_ip: &str,
    req: TokenRequest,
) -> Response {
    let Some(old_refresh) = &req.refresh_token else {
        return token_error("invalid_request", "missing refresh_token");
    };
    let Some(client_id) = &req.client_id else {
        return token_error("invalid_request", "missing client_id");
    };

    // Verify client still exists
    if !state.client_exists(client_id) {
        warn!(client_ip, client_id, "token refresh: client_id not found");
        return token_error("invalid_client", "unknown client_id");
    }

    // Rotate the refresh token (returns new access + refresh tokens)
    let Some((new_access, new_refresh)) = state.rotate_refresh_token(old_refresh, client_id) else {
        warn!(client_ip, client_id, "token refresh: invalid, expired, or wrong-client refresh token");
        return token_error("invalid_grant", "invalid or expired refresh token");
    };

    debug!(client_ip, client_id, "access token refreshed");
    token_success(&new_access, &new_refresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_s256() {
        // RFC 7636 Appendix B example
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_pkce(verifier, challenge));
    }

    #[test]
    fn test_pkce_s256_invalid() {
        assert!(!verify_pkce("wrong_verifier", "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"));
    }
}
