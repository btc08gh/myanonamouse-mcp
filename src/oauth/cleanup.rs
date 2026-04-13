// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::debug;

use super::state::OAuthState;

const CLEANUP_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Spawn a background task that periodically sweeps expired OAuth state.
pub fn spawn_cleanup(state: Arc<OAuthState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        // The first tick fires immediately — skip it so we don't sweep on startup.
        interval.tick().await;

        loop {
            interval.tick().await;
            let result = state.sweep_expired();
            if result.has_any() {
                debug!(
                    auth_codes = result.auth_codes,
                    access_tokens = result.access_tokens,
                    refresh_tokens = result.refresh_tokens,
                    pending_auths = result.pending_auths,
                    stale_clients = result.stale_clients,
                    "OAuth cleanup sweep"
                );
            }
        }
    })
}
