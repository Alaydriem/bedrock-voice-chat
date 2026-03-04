use crate::stream::quic::WebhookReceiver;
use common::structs::{
    channel::ChannelEvents::Rename,
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

use moka::future::Cache;
use std::sync::Arc;

#[patch("/<id>", data = "<name>")]
pub async fn channel_rename<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
    >,
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

    let lock = channel_cache.lock_arc().await;
    let mut channel = match lock.get(id).await {
        Some(channel) => channel,
        None => {
            return status::Custom(Status::NotFound, Some(Json(false)));
        }
    };

    if !channel.creator.eq(&user) {
        return status::Custom(Status::Unauthorized, Some(Json(false)));
    }

    let new_name = name.0;
    channel.rename(new_name.clone());
    _ = lock.insert(id.to_string(), channel).await;
    drop(lock);

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
