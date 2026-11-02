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

/// The BVC side of the Simple Voice Chat bridge's group links.
///
/// The bridge calls this with the in-game names already resolved, as the control route does for
/// a `ClientAction`. Protected channels are invisible here: never listed, leased or closed.
pub struct GroupLinkService {
    channels: Arc<ChannelCollection>,
    webhook: WebhookReceiver,
    leases: Arc<GroupLinkLeases>,
    feed: Arc<GroupLinkFeed>,
}

impl GroupLinkService {
    pub fn new(
        channels: Arc<ChannelCollection>,
        webhook: WebhookReceiver,
        leases: Arc<GroupLinkLeases>,
        feed: Arc<GroupLinkFeed>,
    ) -> Self {
        Self {
            channels,
            webhook,
            leases,
            feed,
        }
    }

    pub fn feed(&self) -> &GroupLinkFeed {
        &self.feed
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
                    name,
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
            })
            .collect()
    }
}
