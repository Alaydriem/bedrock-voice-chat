pub mod feed;
pub mod leases;

pub use feed::GroupLinkFeed;
pub use leases::GroupLinkLeases;

use std::sync::Arc;

use anyhow::anyhow;
use common::Game;
use common::structs::channel::ChannelCollection;
use common::structs::control::{GroupLinkAction, LinkedChannelView};

use crate::services::ChannelMembershipService;
use crate::stream::quic::WebhookReceiver;
use crate::stream::quic::connection::ConnectionRegistry;

/// The BVC side of the Simple Voice Chat bridge's group links.
///
/// The bridge calls this with the in-game names already resolved, as the control route does for
/// a `ClientAction`. Protected channels are invisible here: never listed, leased or closed.
pub struct GroupLinkService {
    channels: Arc<ChannelCollection>,
    webhook: WebhookReceiver,
    leases: Arc<GroupLinkLeases>,
    feed: Arc<GroupLinkFeed>,
    // Absent before the QUIC plane is up. Every member then reads as holding a client.
    registry: Option<Arc<ConnectionRegistry>>,
}

impl GroupLinkService {
    pub fn new(
        channels: Arc<ChannelCollection>,
        webhook: WebhookReceiver,
        leases: Arc<GroupLinkLeases>,
        feed: Arc<GroupLinkFeed>,
        registry: Option<Arc<ConnectionRegistry>>,
    ) -> Self {
        Self {
            channels,
            webhook,
            leases,
            feed,
            registry,
        }
    }

    /// The longest name a linked channel takes, in characters.
    pub const MAX_NAME_CHARS: usize = 64;

    /// What a linked channel is called when the bridge sends no usable name.
    pub const UNNAMED: &'static str = "Group";

    pub fn feed(&self) -> &GroupLinkFeed {
        &self.feed
    }

    // The name comes from a Simple Voice Chat group a player typed. It is cut and never
    // refused: the bridge tries a refused create again on every resync.
    fn linked_name(name: &str) -> String {
        let name: String = name.trim().chars().take(Self::MAX_NAME_CHARS).collect();
        let name = name.trim_end();
        if name.is_empty() {
            Self::UNNAMED.to_string()
        } else {
            name.to_string()
        }
    }

    /// `Some(id)` for `CreateLinked`, `None` for every other action.
    pub async fn apply(&self, action: GroupLinkAction) -> anyhow::Result<Option<String>> {
        match action {
            GroupLinkAction::CreateLinked {
                creator,
                name,
                members_only,
            } => {
                let id = ChannelMembershipService::create_linked(
                    &self.channels,
                    &self.webhook,
                    Self::linked_name(&name),
                    Game::Minecraft.membership_key(&creator),
                    members_only,
                )
                .await;
                Ok(Some(id))
            }
            GroupLinkAction::Close { channel } => {
                let Some(existing) = self.channels.get(&channel).await else {
                    return Ok(None);
                };
                if existing.is_protected() {
                    return Err(anyhow!("a protected group is not linked and cannot be closed"));
                }
                ChannelMembershipService::close(
                    &self.channels,
                    &self.webhook,
                    &existing.creator,
                    &channel,
                )
                .await;
                Ok(None)
            }
            GroupLinkAction::Lease { channels } => {
                let mut linkable = Vec::with_capacity(channels.len());
                for id in channels {
                    if self
                        .channels
                        .get(&id)
                        .await
                        .is_some_and(|channel| !channel.is_protected())
                    {
                        linkable.push(id);
                    }
                }
                self.leases.renew(&linkable);
                Ok(None)
            }
        }
    }

    pub fn channels(&self) -> Vec<LinkedChannelView> {
        self.channels
            .list()
            .into_iter()
            .filter(|channel| !channel.is_protected())
            .map(|channel| LinkedChannelView {
                id: channel.id(),
                name: channel.name.clone(),
                players: channel
                    .players
                    .iter()
                    .map(|player| player.gamertag().to_string())
                    .collect(),
                members_only: channel.is_members_only(),
                clientless: self
                    .registry
                    .as_ref()
                    .map(|registry| {
                        channel
                            .players
                            .iter()
                            .filter(|player| !registry.has_live_client(&player.to_string()))
                            .map(|player| player.gamertag().to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect()
    }
}
