use common::curia;
use common::errors::ProtectRefusal;
use common::structs::channel::{Channel, ChannelCollection, ChannelEvents, GroupCode};
use common::structs::packet::{
    ChannelEventPacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::stream::quic::WebhookReceiver;

/// The single mutate-and-fan path for channel membership, shared by the mTLS
/// channel routes and the `ClientAction` control plane. Each method mutates
/// `ChannelCollection` and fans the matching `ChannelEvent`, which the QUIC
/// `CacheManager` mirrors into `player_channel` for audio routing.
pub struct ChannelMembershipService;

impl ChannelMembershipService {
    /// How many codes are drawn before one is accepted without checking. The space is
    /// 32^8, so a run of this length against a live collection does not happen; the
    /// bound exists so a pathological collection cannot spin here.
    const CODE_TRIES: usize = 8;

    /// Creates an (empty) channel and fans a `Create`. Returns the new share code.
    pub async fn create(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        name: String,
        creator: common::PlayerIdentity,
    ) -> String {
        let channel = Channel::with_id(Self::free_code(channels).await, name, creator);
        Self::insert_and_fan(channels, webhook, channel).await
    }

    /// Creates an empty end-to-end encrypted channel and fans a `Create`. The server records
    /// only that it is protected; it never sees the group's password or keys.
    pub async fn create_protected(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        name: String,
        creator: common::PlayerIdentity,
    ) -> String {
        let channel =
            Channel::with_id(Self::free_code(channels).await, name, creator).into_protected();
        Self::insert_and_fan(channels, webhook, channel).await
    }

    /// Turns an open channel into a protected one and fans a `Rename` with the unchanged name,
    /// which every client already decodes and answers by re-reading the channel list.
    ///
    /// Only the creator may do this, and only while nobody else is in the channel: a member who
    /// joined an open group holds no keys and would be cut off without being told why.
    pub async fn protect(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        caller: &common::PlayerIdentity,
        channel_id: &str,
    ) -> Result<(), ProtectRefusal> {
        channels.protect_if_alone(channel_id, caller).await?;
        let Some(channel) = channels.get(channel_id).await else {
            return Err(ProtectRefusal::NotFound);
        };
        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Rename,
                caller.clone(),
                channel_id.to_string(),
                Some(channel.name),
                Some(channel.creator),
            ),
        )
        .await;
        Ok(())
    }

    async fn insert_and_fan(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        channel: Channel,
    ) -> String {
        let creator = channel.creator.clone();
        let id = channel.id();
        let channel_name = channel.name.clone();
        channels.insert(channel).await;
        Self::fan(
            webhook,
            ChannelEventPacket::new(
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
        member: &common::PlayerIdentity,
        channel_id: &str,
    ) -> bool {
        let Some(channel) = channels.get(channel_id).await else {
            return false;
        };
        channels.add_player_to_channel(member, channel_id).await;
        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Join,
                member.clone(),
                channel_id.to_string(),
                Some(channel.name.clone()),
                Some(channel.creator),
            ),
        )
        .await;
        true
    }

    /// `join` for a channel that must still be open: a plain join never lands in a protected
    /// channel, even one protected a moment ago. Returns `false` (no mutation, no fan) when the
    /// channel does not exist or is protected.
    pub async fn join_open(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        member: &common::PlayerIdentity,
        channel_id: &str,
    ) -> bool {
        if !channels.add_player_to_open_channel(member, channel_id).await {
            return false;
        }
        let Some(channel) = channels.get(channel_id).await else {
            return false;
        };
        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Join,
                member.clone(),
                channel_id.to_string(),
                Some(channel.name.clone()),
                Some(channel.creator),
            ),
        )
        .await;
        true
    }

    /// Removes a member and fans a `Leave`. When `close_if_empty`, a channel left
    /// with no members is removed and a `Delete` fanned.
    pub async fn leave(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        member: &common::PlayerIdentity,
        channel_id: &str,
        close_if_empty: bool,
    ) {
        // Read before the mutation: closing an empty channel removes it, and the owner has
        // to be in hand for the Delete that follows.
        let owner = channels
            .get(channel_id)
            .await
            .map(|channel| (channel.name.clone(), channel.creator));

        channels
            .remove_player_from_channel(member, channel_id)
            .await;

        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Leave,
                member.clone(),
                channel_id.to_string(),
                owner.as_ref().map(|(name, _)| name.clone()),
                owner.as_ref().map(|(_, creator)| creator.clone()),
            ),
        )
        .await;

        if close_if_empty
            && let Some(ch) = channels.get(channel_id).await
            && ch.players.is_empty()
        {
            Self::close(channels, webhook, member, channel_id).await;
        }
    }

    /// Fans a `Leave` for a member the server already removed, as when their connection closed.
    ///
    /// Announced as the channel API rather than as the member: the departed connection did not
    /// send it, and a `ChannelEvent` carrying a player's connection is refused as a client
    /// trying to rewrite membership.
    pub async fn announce_departure(
        webhook: &WebhookReceiver,
        member: &common::PlayerIdentity,
        channel_id: &str,
        creator: common::PlayerIdentity,
    ) {
        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Leave,
                member.clone(),
                channel_id.to_string(),
                None,
                Some(creator),
            ),
        )
        .await;
    }

    /// Removes a channel and fans a `Delete` naming `actor`. Returns `false` (no mutation,
    /// no fan) when the channel does not exist.
    ///
    /// `actor` is whoever the removal is attributed to: the member whose departure emptied
    /// the channel, or — where nobody acted at all, as when the reaper closes an abandoned
    /// channel — its creator.
    pub async fn close(
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
        actor: &common::PlayerIdentity,
        channel_id: &str,
    ) -> bool {
        let Some(channel) = channels.get(channel_id).await else {
            return false;
        };
        channels.remove(channel_id).await;
        Self::fan(
            webhook,
            ChannelEventPacket::new(
                ChannelEvents::Delete,
                actor.clone(),
                channel_id.to_string(),
                Some(channel.name),
                Some(channel.creator),
            ),
        )
        .await;
        true
    }

    /// A share code no live channel is using.
    async fn free_code(channels: &ChannelCollection) -> String {
        let mut code = GroupCode::generate();
        for _ in 0..Self::CODE_TRIES {
            if channels.get(&code).await.is_none() {
                return code;
            }
            code = GroupCode::generate();
        }
        code
    }

    async fn fan(webhook: &WebhookReceiver, event: ChannelEventPacket) {
        let packet = QuicNetworkPacket {
            sender: Some(PacketSender::for_service(PacketSender::CHANNEL_API)),
            packet_type: PacketType::ChannelEvent,
            data: QuicNetworkPacketData::ChannelEvent(event),
                    // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };
        if let Err(e) = webhook.send_packet(packet).await {
            curia::error!("Failed to fan channel event: {}", e);
        }
    }
}
