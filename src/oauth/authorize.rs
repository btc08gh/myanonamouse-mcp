// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use tracing::{debug, warn};

use super::middleware::extract_client_ip;
use super::state::{OAuthState, PendingAuth, generate_token};

// ---------------------------------------------------------------------------
// GET /authorize — query parameters
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
    // Accept and ignore per the guide
    #[allow(dead_code)]
    resource: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
}

// ---------------------------------------------------------------------------
// POST /authorize — form fields
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AuthorizeForm {
    nonce: String,
    action: String,
    password: Option<String>,
}

// ---------------------------------------------------------------------------
// HTML helpers
// ---------------------------------------------------------------------------

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn error_page(status: StatusCode, title: &str, message: &str) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title} — MyAnonamouse MCP</title>
  <style>
    body {{ font-family: system-ui, -apple-system, sans-serif; background: #f5f5f5; display: flex; justify-content: center; padding-top: 80px; }}
    .card {{ background: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); padding: 32px; max-width: 420px; width: 100%; }}
    h1 {{ margin-top: 0; font-size: 1.3em; color: #b91c1c; }}
    p {{ color: #555; line-height: 1.5; }}
  </style>
</head>
<body>
  <div class="card">
    <h1>{title}</h1>
    <p>{message}</p>
  </div>
</body>
</html>"#,
        title = html_escape(title),
        message = html_escape(message),
    );
    (status, Html(html)).into_response()
}

fn redirect_error(redirect_uri: &str, error: &str, description: &str, state: Option<&str>) -> Response {
    let mut url = url::Url::parse(redirect_uri).expect("redirect_uri was validated at registration");
    url.query_pairs_mut()
        .append_pair("error", error)
        .append_pair("error_description", description);
    if let Some(s) = state {
        url.query_pairs_mut().append_pair("state", s);
    }
    Redirect::to(url.as_str()).into_response()
}

fn consent_page(client_name: &str, nonce: &str, requires_password: bool, error_message: Option<&str>) -> String {
    let error_banner = match error_message {
        Some(msg) => format!(
            r#"<div style="background:#fef2f2;border:1px solid #fca5a5;border-radius:4px;padding:10px 14px;margin-bottom:16px;color:#b91c1c;font-size:0.95em">{}</div>"#,
            html_escape(msg)
        ),
        None => String::new(),
    };

    let password_field = if requires_password {
        r#"<div style="margin-bottom:16px">
            <label for="password" style="display:block;margin-bottom:4px;font-weight:600">Access Code</label>
            <input type="password" id="password" name="password" required
                   style="width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;box-sizing:border-box"
                   placeholder="Enter the server access code">
          </div>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Authorize — MyAnonamouse MCP</title>
  <style>
    body {{ font-family: system-ui, -apple-system, sans-serif; background: #f5f5f5; display: flex; justify-content: center; padding-top: 80px; }}
    .card {{ background: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); padding: 32px; max-width: 420px; width: 100%; }}
    h1 {{ margin-top: 0; font-size: 1.3em; }}
    .client-name {{ font-weight: 600; color: #333; }}
    .buttons {{ display: flex; gap: 12px; margin-top: 20px; }}
    .buttons button {{ flex: 1; padding: 10px; border: none; border-radius: 4px; font-size: 1em; cursor: pointer; }}
    .allow {{ background: #2563eb; color: white; }}
    .allow:hover {{ background: #1d4ed8; }}
    .deny {{ background: #e5e7eb; color: #333; }}
    .deny:hover {{ background: #d1d5db; }}
  </style>
</head>
<body>
  <div class="card">
    <h1>Authorize Application</h1>
    <p><span class="client-name">{client_name}</span> wants to access your MyAnonamouse MCP server.</p>
    <form method="POST" action="/authorize">
      <input type="hidden" name="nonce" value="{nonce}">
      {error_banner}
      {password_field}
      <div class="buttons">
        <button type="submit" name="action" value="deny" class="deny">Deny</button>
        <button type="submit" name="action" value="allow" class="allow">Allow</button>
      </div>
    </form>
  </div>
</body>
</html>"#,
        client_name = html_escape(client_name),
        nonce = html_escape(nonce),
        error_banner = error_banner,
        password_field = password_field,
    )
}

// ---------------------------------------------------------------------------
// GET /authorize
// ---------------------------------------------------------------------------

pub async fn handle_authorize_get(
    State(state): State<Arc<OAuthState>>,
    headers: axum::http::HeaderMap,
    Query(params): Query<AuthorizeQuery>,
) -> Response {
    let client_ip = extract_client_ip(&headers);

    // Validate client_id and redirect_uri before we can redirect errors
    let Some(client_id) = &params.client_id else {
        return error_page(StatusCode::BAD_REQUEST, "Invalid Request", "The authorization request is missing a client ID. Please return to the application and try again.");
    };

    let Some((registered_uris, client_name_opt, _)) = state.get_client(client_id) else {
        warn!(client_ip, client_id, "authorize: unknown client_id");
        return error_page(StatusCode::BAD_REQUEST, "Unknown Application", "The application that sent you here is not recognized. Please return to the application and try again.");
    };

    let Some(redirect_uri) = &params.redirect_uri else {
        return error_page(StatusCode::BAD_REQUEST, "Invalid Request", "The authorization request is missing a redirect URI. Please return to the application and try again.");
    };

    // Exact match per OAuth 2.1
    if !registered_uris.contains(redirect_uri) {
        warn!(client_ip, client_id, redirect_uri, "authorize: redirect_uri mismatch");
        return error_page(StatusCode::BAD_REQUEST, "Invalid Request", "The redirect URI does not match any registered URI for this application. Please return to the application and try again.");
    }

    // From here on, we can redirect errors to the redirect_uri
    let state_param = params.state.as_deref();

    if params.response_type.as_deref() != Some("code") {
        return redirect_error(redirect_uri, "unsupported_response_type", "only response_type=code is supported", state_param);
    }

    if params.code_challenge.is_none() {
        return redirect_error(redirect_uri, "invalid_request", "PKCE code_challenge is required", state_param);
    }

    if params.code_challenge_method.as_deref() != Some("S256") {
        return redirect_error(redirect_uri, "invalid_request", "code_challenge_method must be S256", state_param);
    }

    // Create a pending authorization with CSRF nonce
    let nonce = generate_token();
    let pending = PendingAuth {
        client_id: client_id.clone(),
        redirect_uri: redirect_uri.clone(),
        code_challenge: params.code_challenge.clone().unwrap(),
        state: params.state.clone(),
        created_at: std::time::Instant::now(),
    };

    if let Err(msg) = state.insert_pending_auth(nonce.clone(), pending) {
        warn!(client_ip, "authorize: {msg}");
        return redirect_error(redirect_uri, "server_error", msg, state_param);
    }

    let client_name = client_name_opt.unwrap_or_else(|| client_id.clone());

    let requires_password = state.api_token.is_some();
    debug!(client_ip, client_id, "showing consent page");
    Html(consent_page(&client_name, &nonce, requires_password, None)).into_response()
}

// ---------------------------------------------------------------------------
// POST /authorize
// ---------------------------------------------------------------------------

pub async fn handle_authorize_post(
    State(state): State<Arc<OAuthState>>,
    headers: axum::http::HeaderMap,
    Form(form): Form<AuthorizeForm>,
) -> Response {
    let client_ip = extract_client_ip(&headers);

    // Look up the pending auth by CSRF nonce (consumes it — single use)
    let Some(pending) = state.take_pending_auth(&form.nonce) else {
        warn!(client_ip, "authorize POST: invalid or expired nonce");
        return error_page(StatusCode::BAD_REQUEST, "Session Expired", "Your authorization session has expired or is invalid. Please return to the application and try again.");
    };

    let state_param = pending.state.as_deref();

    // User clicked Deny
    if form.action != "allow" {
        debug!(client_ip, client_id = pending.client_id, "user denied authorization");
        return redirect_error(&pending.redirect_uri, "access_denied", "user denied the request", state_param);
    }

    // Validate password if required
    if let Some(expected) = &state.api_token {
        let provided = form.password.as_deref().unwrap_or("");
        use subtle::ConstantTimeEq;
        let matches: bool = provided.as_bytes().ct_eq(expected.as_bytes()).into();
        if !matches {
            warn!(client_ip, client_id = pending.client_id, "authorize: wrong access code");

            // Re-create pending auth with a fresh nonce so the user can retry
            let new_nonce = generate_token();
            let new_pending = PendingAuth {
                client_id: pending.client_id.clone(),
                redirect_uri: pending.redirect_uri,
                code_challenge: pending.code_challenge,
                state: pending.state,
                created_at: std::time::Instant::now(),
            };
            if let Err(msg) = state.insert_pending_auth(new_nonce.clone(), new_pending) {
                warn!(client_ip, "authorize: {msg}");
                return error_page(StatusCode::SERVICE_UNAVAILABLE, "Server Busy", "Too many pending authorization requests. Please try again later.");
            }

            let client_name = state
                .get_client(&pending.client_id)
                .and_then(|(_, name, _)| name)
                .unwrap_or_else(|| pending.client_id.clone());

            return Html(consent_page(
                &client_name,
                &new_nonce,
                true,
                Some("Incorrect access code. Please try again."),
            ))
            .into_response();
        }
    }

    // Issue authorization code
    let code = state.insert_auth_code(
        pending.client_id.clone(),
        pending.redirect_uri.clone(),
        pending.code_challenge,
    );

    debug!(client_ip, client_id = pending.client_id, "authorization code issued");

    // Redirect with code and state
    let mut url = url::Url::parse(&pending.redirect_uri).expect("redirect_uri was validated at registration");
    url.query_pairs_mut().append_pair("code", &code);
    if let Some(s) = state_param {
        url.query_pairs_mut().append_pair("state", s);
    }

    Redirect::to(url.as_str()).into_response()
}
