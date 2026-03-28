use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::ChannelEvents::Rename,
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, serde::json::Json, State};
use crate::http::openapi::CustomJsonResponse;
use rocket_okapi::openapi;

#[openapi(tag = "Channels")]
#[patch("/<id>", data = "<name>")]
pub async fn channel_rename(
    identity: Certificate<'_>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
    name: Json<String>,
) -> CustomJsonResponse<bool> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return CustomJsonResponse::error(Status::Forbidden);
        }
    };

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

    let new_name = name.0;
    channel_collection.rename(id, new_name.clone()).await;

    let packet = QuicNetworkPacket {
        owner: Some(PacketOwner {
            name: String::from("channel_api"),
            client_id: vec![0u8; 0],
        }),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new_full(
            Rename,
            user,
            id.to_string(),
            Some(new_name),
            None,
        )),
    };

    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send channel rename packet to QUIC server: {}", e);
    }

    CustomJsonResponse::ok(true)
}
