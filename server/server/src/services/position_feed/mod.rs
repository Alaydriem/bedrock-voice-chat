mod grid_cell;
mod world_index;

pub use grid_cell::GridCell;
pub use world_index::WorldIndex;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use common::PlayerEnum;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::stream::quic::CacheManager;

/// Builds the world index once per tick and publishes it to every open feed.
///
/// One pass, not one per socket. The route this replaced walked the entire player cache for
/// each observer on every tick, which is observers times players — quadratic, and quadratic
/// on the HTTP runtime. Here the walk is linear in players and each socket does a lookup.
///
/// Published over a `watch` rather than handed out: a socket wants the latest picture, never
/// a queue of stale ones, and a slow socket must not hold a tick open.
pub struct PositionFeedService {
    tx: watch::Sender<Arc<WorldIndex>>,
    rx: watch::Receiver<Arc<WorldIndex>>,
    cell_size: f32,
}

impl PositionFeedService {
    /// Matches the feed's send cadence. Sampling faster than sockets emit would build
    /// pictures nobody reads.
    pub const TICK: Duration = Duration::from_millis(500);

    pub fn new_shared(cell_size: f32) -> Arc<Self> {
        let (tx, rx) = watch::channel(Arc::new(WorldIndex::empty(cell_size)));
        Arc::new(Self { tx, rx, cell_size })
    }

    /// The most recent picture. Never absent: an empty index reads as an empty world, which
    /// is what an observer who is not in the game yet should see.
    pub fn latest(&self) -> Arc<WorldIndex> {
        self.rx.borrow().clone()
    }

    /// A receiver that wakes when the next picture is built.
    ///
    /// Sockets wait on this rather than running their own timer. Two independent tickers at the
    /// same period drift into an arbitrary phase relationship, so a socket could sample an
    /// index up to a whole tick after it was published and a player walking into range would
    /// wait out that skew on top of the rebuild itself. Waking on the write means every socket
    /// emits the picture it was built for.
    pub fn subscribe(&self) -> watch::Receiver<Arc<WorldIndex>> {
        self.rx.clone()
    }

    /// Rebuild the index on a fixed cadence until cancelled.
    pub fn spawn(self: Arc<Self>, cache_manager: CacheManager, cancel: CancellationToken) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Self::TICK);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = ticker.tick() => self.rebuild(&cache_manager).await,
                }
            }
        });
    }

    async fn rebuild(&self, cache_manager: &CacheManager) {
        let players = cache_manager.players().inner_arc();
        let world: Vec<PlayerEnum> = players.iter().map(|(_, player)| player).collect();

        // Voice connections are tracked by the QUIC registry, not by the position cache the
        // mod feeds — which is exactly what makes "in the world, not on voice" answerable.
        let on_voice: HashSet<String> = match cache_manager.get_connection_registry() {
            Some(registry) => registry.on_voice_names(),
            None => HashSet::new(),
        };

        let index = WorldIndex::build(world, on_voice, self.cell_size);
        // A send error means every reader is gone, which cannot happen while this service is
        // managed — and if it did, there would be nobody to tell.
        let _ = self.tx.send(Arc::new(index));
    }
}
