use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channels::ChannelEvents::Rename,
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

#[patch("/<id>", data = "<name>")]
pub async fn channel_rename<'r>(
    identity: Certificate<'r>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    id: &str,
    name: Json<String>,
) -> status::Custom<Option<Json<bool>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let channel = match cache_manager.get_channel(id).await {
        Some(channel) => channel,
        None => {
            return status::Custom(Status::NotFound, Some(Json(false)));
        }
    };

    if !channel.creator.eq(&user) {
        return status::Custom(Status::Unauthorized, Some(Json(false)));
    }

    let new_name = name.0;
    cache_manager.rename_channel(id, new_name.clone()).await;

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

    status::Custom(Status::Ok, Some(Json(true)))
}
