use common::structs::channel::ChannelCollection;
use common::structs::control::{ClientAction, ClientActionType};
use common::structs::packet::{
    ClientActionPacket, PacketDirection, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::services::ChannelMembershipService;
use crate::stream::quic::WebhookReceiver;
use crate::stream::quic::connection_registry::ConnectionRegistry;

/// Applies inbound `ClientAction`s. Self/preference actions are delivered back to
/// the authenticated actor's own connection as a ClientBound `ClientAction`; group
/// actions go through `ChannelMembershipService`. The wire `action.id` is never
/// trusted for routing — the authenticated actor the caller supplies is
/// authoritative.
pub struct ClientActionService;

impl ClientActionService {
    pub fn new() -> Self {
        Self
    }

    /// Delivers a self/preference action to the authenticated actor's own
    /// connection. `actor_name` (the authenticated identity) is authoritative; the
    /// wire `action.id` is overwritten with it. Returns whether a live connection
    /// received it.
    pub fn route_self(
        &self,
        action: &ClientAction,
        actor_name: &str,
        registry: &ConnectionRegistry,
    ) -> bool {
        let packet = ClientActionPacket::new(
            ClientAction {
                id: actor_name.to_string(),
                action: action.action.clone(),
            },
            PacketDirection::ClientBound,
        );
        let envelope = QuicNetworkPacket {
            packet_type: PacketType::ClientAction,
            owner: None,
            data: QuicNetworkPacketData::ClientAction(packet),
        };
        registry.send_to_player(actor_name, &envelope)
    }

    /// Applies a group action for the authenticated actor (cert-CN form
    /// `game:gamertag`). Returns the new nanoid for `CreateGroup`; errors when a
    /// `JoinGroup` targets a channel that does not exist (never creates phantom
    /// membership). `LeaveGroup` closes any channel it empties.
    pub async fn route_group(
        &self,
        action: &ClientActionType,
        actor_cn: &str,
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
    ) -> anyhow::Result<Option<String>> {
        match action {
            ClientActionType::CreateGroup => {
                let id = ChannelMembershipService::create(
                    channels,
                    webhook,
                    format!("{actor_cn} group"),
                    actor_cn.to_string(),
                )
                .await;
                ChannelMembershipService::join(channels, webhook, actor_cn.to_string(), &id).await;
                Ok(Some(id))
            }
            ClientActionType::JoinGroup { channel } => {
                if ChannelMembershipService::join(channels, webhook, actor_cn.to_string(), channel)
                    .await
                {
                    Ok(None)
                } else {
                    anyhow::bail!("channel does not exist: {channel}")
                }
            }
            ClientActionType::LeaveGroup => {
                for cid in channels.get_player_channels(actor_cn) {
                    ChannelMembershipService::leave(
                        channels,
                        webhook,
                        actor_cn.to_string(),
                        &cid,
                        true,
                    )
                    .await;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

impl Default for ClientActionService {
    fn default() -> Self {
        Self::new()
    }
}
