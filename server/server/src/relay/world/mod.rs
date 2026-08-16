pub mod state;

pub use state::WorldWatchState;

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::relay::GrantTable;
use crate::stream::quic::CacheManager;

// Reports which relay worlds this server is hosting.
//
// A relay world id is chosen by the game-side mod and never persisted, so it
// exists nowhere an operator can read it except in live presence. The log line
// is what makes it recoverable after the fact; the `relay worlds` command is
// what makes it available on demand.
pub struct RelayWorldWatch;

impl RelayWorldWatch {
    // Presence entries live 15 seconds, so a shorter tick reports churn rather
    // than change. This is a log line, not a metric.
    const TICK: Duration = Duration::from_secs(30);

    pub fn spawn(
        caches: CacheManager,
        grants: Arc<GrantTable>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let started = Instant::now();
            let mut state = WorldWatchState::new();
            let mut ticker = tokio::time::interval(Self::TICK);

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = ticker.tick() => {}
                }

                let live: Vec<String> = caches
                    .relay_world_populations()
                    .into_iter()
                    .map(|(world, _)| world)
                    .collect();

                if let Some(worlds) = state.observe(&live) {
                    tracing::info!(worlds = ?worlds, "hosting relay worlds");
                }

                for (label, world) in
                    state.unwarned_missing(&grants.configured_worlds(), started.elapsed())
                {
                    tracing::warn!(
                        peer = %label,
                        world = %world,
                        "a peer block filters on a world no local player has been seen in; \
                         that peer will carry nothing"
                    );
                }
            }
        })
    }
}
