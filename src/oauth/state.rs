// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::Rng;

// ---------------------------------------------------------------------------
// Token/code generation
// ---------------------------------------------------------------------------

/// Generate a 256-bit cryptographically random token, returned as base64url (no padding).
pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(bytes)
}

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

pub struct ClientRecord {
    pub redirect_uris: Vec<String>,
    pub client_name: Option<String>,
    pub created_at: Instant,
    /// Set to `true` once a token has been issued for this client.
    pub authorized: bool,
}

pub struct AuthCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub expires_at: Instant,
}

pub struct AccessToken {
    pub client_id: String,
    pub expires_at: Instant,
}

pub struct RefreshToken {
    pub client_id: String,
    pub expires_at: Instant,
    /// If this token was superseded by rotation, the time it was replaced.
    /// It remains valid for a 30-second grace period after this instant.
    pub superseded_at: Option<Instant>,
}

pub struct PendingAuth {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub state: Option<String>,
    pub created_at: Instant,
}

// ---------------------------------------------------------------------------
// Caps and lifetimes
// ---------------------------------------------------------------------------

const MAX_CLIENTS: usize = 100;
const MAX_PENDING_AUTHS: usize = 1000;
const UNAUTHED_CLIENT_TTL: Duration = Duration::from_secs(15 * 60);
const AUTH_CODE_TTL: Duration = Duration::from_secs(10 * 60);
const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(3600);
const REFRESH_TOKEN_TTL: Duration = Duration::from_secs(24 * 3600);
const PENDING_AUTH_TTL: Duration = Duration::from_secs(5 * 60);
const REFRESH_GRACE_PERIOD: Duration = Duration::from_secs(30);

// Re-export lifetimes needed by other modules
pub const ACCESS_TOKEN_LIFETIME_SECS: u64 = 3600;

// ---------------------------------------------------------------------------
// OAuthState
// ---------------------------------------------------------------------------

pub struct OAuthState {
    pub issuer: String,
    pub api_token: Option<String>,
    clients: Mutex<HashMap<String, ClientRecord>>,
    auth_codes: Mutex<HashMap<String, AuthCode>>,
    pending_auths: Mutex<HashMap<String, PendingAuth>>,
    access_tokens: Mutex<HashMap<String, AccessToken>>,
    refresh_tokens: Mutex<HashMap<String, RefreshToken>>,
}

impl OAuthState {
    pub fn new(issuer: String, api_token: Option<String>) -> Self {
        Self {
            issuer: issuer.trim_end_matches('/').to_string(),
            api_token,
            clients: Mutex::new(HashMap::new()),
            auth_codes: Mutex::new(HashMap::new()),
            pending_auths: Mutex::new(HashMap::new()),
            access_tokens: Mutex::new(HashMap::new()),
            refresh_tokens: Mutex::new(HashMap::new()),
        }
    }

    // -- Clients --

    pub fn register_client(
        &self,
        redirect_uris: Vec<String>,
        client_name: Option<String>,
    ) -> Result<String, &'static str> {
        let mut clients = self.clients.lock().unwrap();
        if clients.len() >= MAX_CLIENTS {
            return Err("client registration limit reached");
        }
        let client_id = generate_token();
        clients.insert(
            client_id.clone(),
            ClientRecord {
                redirect_uris,
                client_name,
                created_at: Instant::now(),
                authorized: false,
            },
        );
        Ok(client_id)
    }

    pub fn get_client(&self, client_id: &str) -> Option<(Vec<String>, Option<String>, bool)> {
        let clients = self.clients.lock().unwrap();
        clients
            .get(client_id)
            .map(|c| (c.redirect_uris.clone(), c.client_name.clone(), c.authorized))
    }

    pub fn mark_client_authorized(&self, client_id: &str) {
        let mut clients = self.clients.lock().unwrap();
        if let Some(c) = clients.get_mut(client_id) {
            c.authorized = true;
        }
    }

    pub fn client_exists(&self, client_id: &str) -> bool {
        self.clients.lock().unwrap().contains_key(client_id)
    }

    // -- Pending authorizations --

    pub fn insert_pending_auth(&self, nonce: String, pending: PendingAuth) -> Result<(), &'static str> {
        let mut auths = self.pending_auths.lock().unwrap();
        if auths.len() >= MAX_PENDING_AUTHS {
            return Err("too many pending authorizations");
        }
        auths.insert(nonce, pending);
        Ok(())
    }

    /// Remove and return the pending auth for the given nonce (single-use).
    pub fn take_pending_auth(&self, nonce: &str) -> Option<PendingAuth> {
        self.pending_auths.lock().unwrap().remove(nonce)
    }

    // -- Authorization codes --

    pub fn insert_auth_code(&self, client_id: String, redirect_uri: String, code_challenge: String) -> String {
        let code = generate_token();
        let mut codes = self.auth_codes.lock().unwrap();
        codes.insert(
            code.clone(),
            AuthCode {
                client_id,
                redirect_uri,
                code_challenge,
                expires_at: Instant::now() + AUTH_CODE_TTL,
            },
        );
        code
    }

    /// Remove and return the auth code (single-use). Returns None if expired or missing.
    pub fn take_auth_code(&self, code: &str) -> Option<AuthCode> {
        let mut codes = self.auth_codes.lock().unwrap();
        let ac = codes.remove(code)?;
        if Instant::now() > ac.expires_at {
            return None;
        }
        Some(ac)
    }

    // -- Access tokens --

    pub fn insert_access_token(&self, client_id: String) -> String {
        let token = generate_token();
        let mut tokens = self.access_tokens.lock().unwrap();
        tokens.insert(
            token.clone(),
            AccessToken {
                client_id,
                expires_at: Instant::now() + ACCESS_TOKEN_TTL,
            },
        );
        token
    }

    /// Validate an access token. Returns the client_id if valid.
    pub fn validate_access_token(&self, token: &str) -> Option<String> {
        let tokens = self.access_tokens.lock().unwrap();
        let at = tokens.get(token)?;
        if Instant::now() > at.expires_at {
            return None;
        }
        Some(at.client_id.clone())
    }

    // -- Refresh tokens --

    pub fn insert_refresh_token(&self, client_id: String) -> String {
        let token = generate_token();
        let mut tokens = self.refresh_tokens.lock().unwrap();
        tokens.insert(
            token.clone(),
            RefreshToken {
                client_id,
                expires_at: Instant::now() + REFRESH_TOKEN_TTL,
                superseded_at: None,
            },
        );
        token
    }

    /// Consume a refresh token (rotation). Returns a new (access_token, refresh_token)
    /// pair, or None if the token is invalid/expired/past grace period/wrong client.
    pub fn rotate_refresh_token(&self, old_token: &str, client_id: &str) -> Option<(String, String)> {
        let mut tokens = self.refresh_tokens.lock().unwrap();
        let rt = tokens.get_mut(old_token)?;

        // Verify the refresh token belongs to this client BEFORE we mark it superseded.
        if rt.client_id != client_id {
            return None;
        }

        let now = Instant::now();

        // Check expiry
        if now > rt.expires_at {
            tokens.remove(old_token);
            return None;
        }

        // Check grace period for superseded tokens
        if let Some(superseded_at) = rt.superseded_at {
            if now > superseded_at + REFRESH_GRACE_PERIOD {
                tokens.remove(old_token);
                return None;
            }
            // Already superseded but within grace period — reject rather than
            // minting yet another token pair, which would extend the window
            // indefinitely and defeat rotation security.
            return None;
        }

        // Mark old token as superseded (keep for grace period for one retry)
        rt.superseded_at = Some(now);

        // Drop the lock before calling methods that re-acquire it
        drop(tokens);

        let new_access = self.insert_access_token(client_id.to_string());
        let new_refresh = self.insert_refresh_token(client_id.to_string());

        Some((new_access, new_refresh))
    }

    // -- Cleanup --

    /// Sweep all expired entries. Returns counts of swept items per category.
    pub fn sweep_expired(&self) -> SweepResult {
        let now = Instant::now();
        let mut result = SweepResult::default();

        // Expired auth codes
        {
            let mut codes = self.auth_codes.lock().unwrap();
            let before = codes.len();
            codes.retain(|_, c| now < c.expires_at);
            result.auth_codes = before - codes.len();
        }

        // Expired access tokens
        {
            let mut tokens = self.access_tokens.lock().unwrap();
            let before = tokens.len();
            tokens.retain(|_, t| now < t.expires_at);
            result.access_tokens = before - tokens.len();
        }

        // Expired or superseded-past-grace refresh tokens
        {
            let mut tokens = self.refresh_tokens.lock().unwrap();
            let before = tokens.len();
            tokens.retain(|_, t| {
                if now > t.expires_at {
                    return false;
                }
                if let Some(superseded_at) = t.superseded_at {
                    if now > superseded_at + REFRESH_GRACE_PERIOD {
                        return false;
                    }
                }
                true
            });
            result.refresh_tokens = before - tokens.len();
        }

        // Expired pending authorizations
        {
            let mut auths = self.pending_auths.lock().unwrap();
            let before = auths.len();
            auths.retain(|_, a| now < a.created_at + PENDING_AUTH_TTL);
            result.pending_auths = before - auths.len();
        }

        // Stale (unauthed) client registrations
        {
            let mut clients = self.clients.lock().unwrap();
            let before = clients.len();
            clients.retain(|_, c| c.authorized || now < c.created_at + UNAUTHED_CLIENT_TTL);
            result.stale_clients = before - clients.len();
        }

        result
    }
}

#[derive(Default)]
pub struct SweepResult {
    pub auth_codes: usize,
    pub access_tokens: usize,
    pub refresh_tokens: usize,
    pub pending_auths: usize,
    pub stale_clients: usize,
}

impl SweepResult {
    pub fn has_any(&self) -> bool {
        self.auth_codes > 0
            || self.access_tokens > 0
            || self.refresh_tokens > 0
            || self.pending_auths > 0
            || self.stale_clients > 0
    }
}
