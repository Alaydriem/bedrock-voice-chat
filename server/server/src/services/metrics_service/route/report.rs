use std::time::Duration;

use crate::services::metrics_service::interaction::{InteractionRoute, RouteRejection};

/// What one `route_audio_frame` call did, gathered on the routing thread and handed off whole.
///
/// The fan-out loop writes only to this value, which it owns, so no recipient costs a shared
/// counter or a map lookup. Every shared structure is written later by
/// `RouteTelemetry::flush`, off the routing path.
#[derive(Debug)]
pub struct RouteFrameReport {
    rejections: [u32; RouteRejection::ALL.len()],
    queue_full_drops: u32,
    // `None` when this frame's deliveries are not player interactions: a sender with no live
    // connection, or a server with no metrics installed. Deliveries are then not collected.
    sender: Option<u64>,
    deliveries: Vec<(InteractionRoute, u64)>,
    duration: Duration,
}

impl RouteFrameReport {
    pub fn new(sender: Option<u64>) -> Self {
        Self {
            rejections: [0; RouteRejection::ALL.len()],
            queue_full_drops: 0,
            sender,
            deliveries: Vec::new(),
            duration: Duration::ZERO,
        }
    }

    pub fn reject(&mut self, reason: RouteRejection) {
        let count = &mut self.rejections[reason.index()];
        *count = count.saturating_add(1);
    }

    // Counted apart from the `RecipientQueueFull` rejection because dashboards already read
    // this drop counter.
    pub fn drop_for_full_queue(&mut self) {
        self.queue_full_drops = self.queue_full_drops.saturating_add(1);
    }

    pub fn deliver(&mut self, route: InteractionRoute, recipient: u64) {
        if self.sender.is_some() {
            self.deliveries.push((route, recipient));
        }
    }

    pub fn finish(&mut self, duration: Duration) {
        self.duration = duration;
    }

    pub fn rejections(&self) -> impl Iterator<Item = (RouteRejection, u32)> + '_ {
        RouteRejection::ALL
            .iter()
            .zip(self.rejections.iter())
            .filter(|(_, count)| **count > 0)
            .map(|(reason, count)| (*reason, *count))
    }

    pub fn queue_full_drops(&self) -> u32 {
        self.queue_full_drops
    }

    pub fn sender(&self) -> Option<u64> {
        self.sender
    }

    pub fn deliveries(&self) -> &[(InteractionRoute, u64)] {
        &self.deliveries
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}
