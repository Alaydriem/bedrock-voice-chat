use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::ChannelEvents::Delete,
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, State};
use crate::http::openapi::CustomJsonResponse;
use rocket_okapi::openapi;

#[openapi(tag = "Channels")]
#[delete("/<id>")]
pub async fn channel_delete(
    identity: Certificate<'_>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
) -> CustomJsonResponse<bool> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return CustomJsonResponse::error(Status::Forbidden);
        }
    };

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
                owner: Some(PacketOwner {
                    name: String::from("channel_api"),
                    client_id: vec![0u8; 0],
                }),
                packet_type: PacketType::ChannelEvent,
                data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new_full(
                    Delete,
                    user,
                    id.to_string(),
                    Some(channel_name),
                    Some(creator),
                )),
            };

            if let Err(e) = webhook_receiver.send_packet(packet).await {
                tracing::error!("Failed to send channel delete packet to QUIC server: {}", e);
            }

            CustomJsonResponse::ok(true)
        }
        None => CustomJsonResponse::custom(Status::NotFound, Some(false)),
    }
}
