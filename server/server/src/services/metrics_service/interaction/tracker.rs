use std::sync::atomic::{AtomicU64, Ordering};

use super::counts::InteractionCounts;
use super::route::InteractionRoute;
use super::window::InteractionWindow;

/// Tumbling-window measurement of who was routed to whom. Delivery is the observable
/// the server has; audition is not, so these are reach figures, not listening figures.
/// The window boundary is the heartbeat tick by construction: the heartbeat task is
/// the only caller of `close_window`, so each sample covers exactly the interval since
/// the last one. Counts are distinct players and therefore do not sum across windows.
pub struct InteractionTracker {
    window: AtomicU64,
    proximity: InteractionWindow,
    channel: InteractionWindow,
    any: InteractionWindow,
}

impl InteractionTracker {
    pub fn new() -> Self {
        Self {
            window: AtomicU64::new(0),
            proximity: InteractionWindow::new(),
            channel: InteractionWindow::new(),
            any: InteractionWindow::new(),
        }
    }

    pub fn hash_name(name: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }

    /// Called from the audio delivery seam once per successfully routed frame.
    /// `route` is the branch that delivered it; the `any` series is maintained
    /// here rather than derived downstream.
    pub fn record_delivery(&self, route: InteractionRoute, sender: u64, recipient: u64) {
        let per_route = match route {
            InteractionRoute::Proximity => &self.proximity,
            InteractionRoute::Channel => &self.channel,
            InteractionRoute::Any => return,
        };
        let window = self.window.load(Ordering::Relaxed);
        let pair = ((sender as u128) << 64) | recipient as u128;
        per_route.record(pair, sender, recipient, window);
        self.any.record(pair, sender, recipient, window);
    }

    pub fn counts(&self, route: InteractionRoute) -> InteractionCounts {
        let window = self.window.load(Ordering::Relaxed);
        match route {
            InteractionRoute::Proximity => self.proximity.counts(window),
            InteractionRoute::Channel => self.channel.counts(window),
            InteractionRoute::Any => self.any.counts(window),
        }
    }

    /// Reads the closing window's figures and opens the next one.
    pub fn close_window(&self) -> [(InteractionRoute, InteractionCounts); 3] {
        let window = self.window.load(Ordering::Relaxed);
        let closed = [
            (InteractionRoute::Proximity, self.proximity.counts(window)),
            (InteractionRoute::Channel, self.channel.counts(window)),
            (InteractionRoute::Any, self.any.counts(window)),
        ];
        self.window.store(window + 1, Ordering::Relaxed);
        self.proximity.clear();
        self.channel.clear();
        self.any.clear();
        closed
    }
}

impl Default for InteractionTracker {
    fn default() -> Self {
        Self::new()
    }
}
