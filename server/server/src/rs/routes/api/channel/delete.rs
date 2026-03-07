use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::ChannelEvents::Delete,
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

/// Deletes a channel if the identity provided with the certificate is the creator
#[delete("/<id>")]
pub async fn channel_delete<'r>(
    identity: Certificate<'r>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
) -> status::Custom<Option<Json<bool>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    match cache_manager.get_channel(id).await {
        Some(channel) => {
            if !channel.creator.eq(&user) {
                return status::Custom(Status::Unauthorized, Some(Json(false)));
            }

            let channel_name = channel.name.clone();
            let creator = channel.creator.clone();

            cache_manager.delete_channel(id).await;

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

            return status::Custom(Status::Ok, Some(Json(true)));
        }
        None => {
            return status::Custom(Status::NotFound, Some(Json(false)));
        }
    }
}
