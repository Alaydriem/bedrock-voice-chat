use common::curia;
use crate::http::openapi::CustomJsonResponse;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::ChannelEvents::Rename,
    packet::{
        ChannelEventPacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use crate::http::guards::PlayerGuard;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

/// Rename a channel.
#[openapi(tag = "Channels")]
#[patch("/<id>", data = "<name>")]
pub async fn channel_rename(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
    name: Json<String>,
) -> CustomJsonResponse<bool> {
    // Derived from the resolved player rather than read off the certificate CN, so a
    // legacy bare-gamertag certificate produces the same canonical identity as a current one.
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let user = guard.player.game.membership_key(&gamertag);

    let channel_collection = cache_manager.get_channel_collection();
    let channel = match channel_collection.get(id).await {
        Some(channel) => channel,
        None => {
            return CustomJsonResponse::custom(Status::NotFound, Some(false));
        }
    };

    if !channel.creator.eq(&user) {
        return CustomJsonResponse::custom(Status::Unauthorized, Some(false));
    }

    // Read before the rename, so the event names the owner the channel had when the
    // caller was authorised against it.
    let creator = channel.creator.clone();

    let new_name = name.0;
    channel_collection.rename(id, new_name.clone()).await;

    let packet = QuicNetworkPacket {
        sender: Some(PacketSender::for_service(PacketSender::CHANNEL_API)),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
            Rename,
            user,
            id.to_string(),
            Some(new_name),
            Some(creator),
        )),
            // Not a server fan-out, so this envelope carries no sequence.
        ..Default::default()
    };

    if let Err(e) = webhook_receiver.send_packet(packet).await {
        curia::error!("Failed to send channel rename packet to QUIC server: {}", e);
    }

    CustomJsonResponse::ok(true)
}
