use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::curia;
use common::structs::channel::ChannelCollection;
use dashmap::DashMap;

use crate::services::ChannelMembershipService;
use crate::stream::quic::WebhookReceiver;
use crate::stream::quic::connection::ConnectionRegistry;

/// Closes channels nobody is connected to any more.
///
/// Four paths empty a channel and only two of them close it: the in-game control plane
/// closes through `leave(close_if_empty: true)`, and the creator closes explicitly through
/// `DELETE /api/channel/<id>`. The desktop client's `Leave` passes `close_if_empty: false`,
/// and a disconnect fans a `Leave` per channel without ever closing what it emptied. What
/// those two leave behind stays listed by `GET /api/channel`, which is the list a group name
/// is resolved against.
///
/// The `ChannelCollection` TTL is not that backstop. It restarts on every membership *write*,
/// so it measures time since the last join or leave rather than time without members: a group
/// whose members sit in it without churn produces no writes and ages out underneath them.
/// Liveness is the quantity that decides whether a group is in use, so it is the one read here.
pub struct ChannelReaperService {
    channels: Arc<ChannelCollection>,
    registry: Arc<ConnectionRegistry>,
    webhook: WebhookReceiver,
    // When each channel was first seen with nobody connected. Kept here rather than on
    // `Channel`, which is a ts-rs DTO the webview receives: server bookkeeping does not
    // belong in a shared type, and a field there would regenerate the binding.
    idle_since: DashMap<String, Instant>,
}

impl ChannelReaperService {
    /// How long a channel may hold no connected member before it is closed.
    pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

    pub fn new(
        channels: Arc<ChannelCollection>,
        registry: Arc<ConnectionRegistry>,
        webhook: WebhookReceiver,
    ) -> Self {
        Self {
            channels,
            registry,
            webhook,
            idle_since: DashMap::new(),
        }
    }

    pub fn new_shared(
        channels: Arc<ChannelCollection>,
        registry: Arc<ConnectionRegistry>,
        webhook: WebhookReceiver,
    ) -> Arc<Self> {
        Arc::new(Self::new(channels, registry, webhook))
    }

    /// One pass over every channel, against the current clock.
    pub async fn sweep(&self) {
        self.sweep_at(Instant::now()).await;
    }

    /// One pass over every channel, against `now`.
    ///
    /// The clock is a parameter so the grace can be exercised without waiting it out.
    pub async fn sweep_at(&self, now: Instant) {
        let channels = self.channels.list();

        // Channels closed by any other path leave a mark behind. Dropped here so a channel
        // id that comes round again starts its grace fresh rather than inheriting one.
        let known: HashSet<String> = channels.iter().map(|channel| channel.id()).collect();
        self.idle_since.retain(|id, _| known.contains(id));

        for channel in channels {
            let id = channel.id();

            let occupied = channel
                .players
                .iter()
                .any(|player| self.registry.has_live_client(&player.to_string()));

            if occupied {
                self.idle_since.remove(&id);
                continue;
            }

            let idle_since = *self.idle_since.entry(id.clone()).or_insert(now);
            if now.saturating_duration_since(idle_since) < Self::IDLE_TIMEOUT {
                continue;
            }

            self.idle_since.remove(&id);
            let creator = channel.creator.clone();
            if ChannelMembershipService::close(&self.channels, &self.webhook, &creator, &id).await {
                curia::info!(
                    "Closed group {} ({}): no connected member for {} minutes",
                    id,
                    channel.name,
                    Self::IDLE_TIMEOUT.as_secs() / 60
                );
            }
        }
    }
}
