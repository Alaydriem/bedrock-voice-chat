use common::structs::channel::{Channel, ChannelCollection, ChannelEvents};
use common::structs::packet::{
    ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::stream::quic::WebhookReceiver;

/// The single mutate-and-fan path for channel membership, shared by the mTLS
/// channel routes and the `ClientAction` control plane. Each method mutates
/// `ChannelCollection` and fans the matching `ChannelEvent`, which the QUIC
/// `CacheManager` mirrors into `player_channel` for audio routing.
pub struct ChannelMembershipService;

impl ChannelMembershipService {
    /// Creates an (empty) channel and fans a `Create`. Returns the new nanoid.
    pub async fn create(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        name: String,
        creator: String,
    ) -> String {
        let channel = Channel::new(name, creator.clone());
        let id = channel.id();
        let channel_name = channel.name.clone();
        channels.insert(channel).await;
        Self::fan(
            webhook,
            ChannelEventPacket::new_full(
                ChannelEvents::Create,
                creator.clone(),
                id.clone(),
                Some(channel_name),
                Some(creator),
            ),
        )
        .await;
        id
    }

    /// Adds a member to an existing channel and fans a `Join`. Returns `false`
    /// (no mutation, no fan) when the channel does not exist.
    pub async fn join(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        member: String,
        channel_id: &str,
    ) -> bool {
        if channels.get(channel_id).await.is_none() {
            return false;
        }
        channels.add_player_to_channel(&member, channel_id).await;
        Self::fan(
            webhook,
            ChannelEventPacket::new(ChannelEvents::Join, member, channel_id.to_string()),
        )
        .await;
        true
    }

    /// Removes a member and fans a `Leave`. When `close_if_empty`, a channel left
    /// with no members is removed and a `Delete` fanned.
    pub async fn leave(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        member: String,
        channel_id: &str,
        close_if_empty: bool,
    ) {
        channels
            .remove_player_from_channel(&member, channel_id)
            .await;
        Self::fan(
            webhook,
            ChannelEventPacket::new(ChannelEvents::Leave, member.clone(), channel_id.to_string()),
        )
        .await;
        if close_if_empty {
            if let Some(ch) = channels.get(channel_id).await {
                if ch.players.is_empty() {
                    channels.remove(channel_id).await;
                    Self::fan(
                        webhook,
                        ChannelEventPacket::new(
                            ChannelEvents::Delete,
                            member,
                            channel_id.to_string(),
                        ),
                    )
                    .await;
                }
            }
        }
    }

    async fn fan(webhook: &WebhookReceiver, event: ChannelEventPacket) {
        let packet = QuicNetworkPacket {
            owner: Some(PacketOwner {
                name: String::from("channel_api"),
                client_id: vec![0u8; 0],
            }),
            packet_type: PacketType::ChannelEvent,
            data: QuicNetworkPacketData::ChannelEvent(event),
                    // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };
        if let Err(e) = webhook.send_packet(packet).await {
            tracing::error!("Failed to fan channel event: {}", e);
        }
    }
}
