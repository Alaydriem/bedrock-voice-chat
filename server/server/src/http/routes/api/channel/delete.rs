use common::curia;
use crate::http::openapi::CustomJsonResponse;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::ChannelEvents::Delete,
    packet::{
        ChannelEventPacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use crate::http::guards::PlayerGuard;
use rocket::{State, http::Status};
use rocket_okapi::openapi;

/// Delete a channel, removing every member.
#[openapi(tag = "Channels")]
#[delete("/<id>")]
pub async fn channel_delete(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
) -> CustomJsonResponse<bool> {
    // Derived from the resolved player rather than read off the certificate CN, so a
    // legacy bare-gamertag certificate produces the same canonical identity as a current one.
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let user = guard.player.game.membership_key(&gamertag);

    let channel_collection = cache_manager.get_channel_collection();
    match channel_collection.get(id).await {
        Some(channel) => {
            if !channel.creator.eq(&user) {
                return CustomJsonResponse::custom(Status::Unauthorized, Some(false));
            }

            let channel_name = channel.name.clone();
            let creator = channel.creator.clone();

            channel_collection.remove(id).await;

            let packet = QuicNetworkPacket {
                sender: Some(PacketSender::for_service(PacketSender::CHANNEL_API)),
                packet_type: PacketType::ChannelEvent,
                data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
                    Delete,
                    user,
                    id.to_string(),
                    Some(channel_name),
                    Some(creator),
                )),
                            // Not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
            };

            if let Err(e) = webhook_receiver.send_packet(packet).await {
                curia::error!("Failed to send channel delete packet to QUIC server: {}", e);
            }

            CustomJsonResponse::ok(true)
        }
        None => CustomJsonResponse::custom(Status::NotFound, Some(false)),
    }
}
