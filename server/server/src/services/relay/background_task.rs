use std::sync::Arc;
use std::time::Duration;

use common::structs::relay::RelayEndpoint;

use super::client::RelayClient;
use super::peer_table::PeerTable;
use super::register_nonce_store::RegisterNonceStore;

const DEFAULT_REGISTER_INTERVAL_SECS: u64 = 60;

const REGISTER_TTL_SECS: u32 = 300;

// Seam for "which relay worlds is this server currently hosting active clients in".
// Backed by the live player cache; kept lock-light so the background task never
// blocks the audio path.
pub trait ActiveWorldsSource: Send + Sync {
    fn active_worlds(&self) -> Vec<String>;
}

// Adapts a closure into an `ActiveWorldsSource` so callers that already hold an
// `Arc` over a cache can supply worlds without a bespoke type.
pub struct FnActiveWorldsSource<F: Fn() -> Vec<String> + Send + Sync>(pub F);

impl<F: Fn() -> Vec<String> + Send + Sync> ActiveWorldsSource for FnActiveWorldsSource<F> {
    fn active_worlds(&self) -> Vec<String> {
        (self.0)()
    }
}

// Periodically registers each active world with the relay and refreshes the
// local `PeerTable` from the relay's scoped lookup. A dedicated `tokio::spawn`
// task — never on the QUIC receive path.
pub struct RelayBackgroundTask {
    client: Arc<RelayClient>,
    peer_table: Arc<PeerTable>,
    self_endpoint: RelayEndpoint,
    worlds: Arc<dyn ActiveWorldsSource>,
    // Where nonces issued by the relay's registration challenge are remembered so
    // this server's `/relay/proof` route can serve them.
    nonces: Arc<RegisterNonceStore>,
    interval: Duration,
}

impl RelayBackgroundTask {
    pub fn new(
        client: Arc<RelayClient>,
        peer_table: Arc<PeerTable>,
        self_endpoint: RelayEndpoint,
        worlds: Arc<dyn ActiveWorldsSource>,
        nonces: Arc<RegisterNonceStore>,
    ) -> Self {
        Self::new_with_interval(
            client,
            peer_table,
            self_endpoint,
            worlds,
            nonces,
            Duration::from_secs(DEFAULT_REGISTER_INTERVAL_SECS),
        )
    }

    pub fn new_with_interval(
        client: Arc<RelayClient>,
        peer_table: Arc<PeerTable>,
        self_endpoint: RelayEndpoint,
        worlds: Arc<dyn ActiveWorldsSource>,
        nonces: Arc<RegisterNonceStore>,
        interval: Duration,
    ) -> Self {
        Self {
            client,
            peer_table,
            self_endpoint,
            worlds,
            nonces,
            interval,
        }
    }

    // True only when the task has never had a successful relay interaction and
    // the latest attempt failed: the relay is presumed unreachable for this
    // process, so the task gives up rather than error-spamming at boot.
    fn should_give_up(connected_once: bool, attempt_failed: bool) -> bool {
        !connected_once && attempt_failed
    }

    // One register+lookup cycle. Reports whether any relay interaction succeeded
    // and the active worlds it processed. A cycle with no active worlds did not
    // contact the relay, so it counts as neither success nor failure.
    pub async fn tick(&self) -> TickOutcome {
        let active = self.worlds.active_worlds();
        self.peer_table.set_active_worlds(active.clone());

        if active.is_empty() {
            return TickOutcome {
                worlds: active,
                attempted: false,
                succeeded: false,
            };
        }

        // The last token whose endpoint-control proof completed via register's
        // reachability callback. It is bound to `self_endpoint`, so the relay
        // accepts it for the lookup gate too. All challenges this cycle are for
        // the same endpoint; any verified one works.
        let mut lookup_token: Option<String> = None;
        let mut succeeded = false;

        for world in &active {
            // Endpoint-control-proven registration: obtain a challenge, remember
            // the nonce so our `/relay/proof` route can serve
            // it for the relay's reachability callback, then register with the
            // proven token.
            let token = match self
                .client
                .request_challenge(self.self_endpoint.clone())
                .await
            {
                Ok(ch) => {
                    self.nonces.remember(&ch.nonce);
                    ch.token
                }
                Err(e) => {
                    tracing::warn!("relay challenge failed for world {}: {}", world, e);
                    continue;
                }
            };

            match self
                .client
                .register(world, self.self_endpoint.clone(), REGISTER_TTL_SECS, &token)
                .await
            {
                Ok(()) => {
                    lookup_token = Some(token);
                    succeeded = true;
                }
                Err(e) => tracing::warn!("relay register failed for world {}: {}", world, e),
            }
        }

        // Lookup is gated on an endpoint-control-proven token. Reuse a token whose
        // proof completed during register; if none verified this cycle, skip
        // lookup (the relay would reject it anyway).
        let Some(token) = lookup_token else {
            return TickOutcome {
                worlds: active,
                attempted: true,
                succeeded,
            };
        };

        match self
            .client
            .lookup(self.self_endpoint.clone(), &active, &token)
            .await
        {
            Ok(map) => {
                for (world, peers) in map {
                    self.peer_table.set_world_peers(&world, peers);
                }
                succeeded = true;
            }
            Err(e) => tracing::warn!("relay lookup failed: {}", e),
        }

        TickOutcome {
            worlds: active,
            attempted: true,
            succeeded,
        }
    }

    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.interval);
        let mut connected_once = false;
        loop {
            ticker.tick().await;
            let outcome = self.tick().await;

            if outcome.succeeded {
                connected_once = true;
                continue;
            }

            let attempt_failed = outcome.attempted && !outcome.succeeded;
            if Self::should_give_up(connected_once, attempt_failed) {
                tracing::error!(
                    "cross-server relay unreachable at {}; cross-server voice disabled for this session",
                    self.self_endpoint.host
                );
                return;
            }
        }
    }
}

// What a single `tick` did against the relay. `attempted` is false when there
// were no active worlds (nothing was sent), distinguishing an idle cycle from a
// real failure so an idle boot never trips the give-up path.
pub struct TickOutcome {
    pub worlds: Vec<String>,
    pub attempted: bool,
    pub succeeded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_source_returns_closure_value() {
        let src = FnActiveWorldsSource(|| vec!["w1".to_string(), "w2".to_string()]);
        assert_eq!(src.active_worlds(), vec!["w1".to_string(), "w2".to_string()]);
    }

    #[test]
    fn fn_source_is_object_safe_via_arc() {
        let src: Arc<dyn ActiveWorldsSource> =
            Arc::new(FnActiveWorldsSource(|| vec!["only".to_string()]));
        assert_eq!(src.active_worlds(), vec!["only".to_string()]);
    }

    #[test]
    fn never_connected_failure_gives_up() {
        assert!(RelayBackgroundTask::should_give_up(false, true));
    }

    #[test]
    fn connected_once_failure_keeps_retrying() {
        assert!(!RelayBackgroundTask::should_give_up(true, true));
    }

    #[test]
    fn never_connected_no_failure_does_not_give_up() {
        assert!(!RelayBackgroundTask::should_give_up(false, false));
    }

    #[test]
    fn connected_once_no_failure_does_not_give_up() {
        assert!(!RelayBackgroundTask::should_give_up(true, false));
    }
}
