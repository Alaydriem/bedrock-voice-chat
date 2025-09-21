use crate::stream::quic::WebhookReceiver;
use common::structs::{
    channel::{Channel, ChannelEvents::Create},
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

use moka::future::Cache;
use std::sync::Arc;

/// Creates a new channel
#[post("/", data = "<name>")]
pub async fn channel_create<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
    >,
    webhook_receiver: &State<WebhookReceiver>,
    name: Json<String>,
) -> status::Custom<Option<Json<String>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let channel = Channel::new(name.0.clone(), user.clone());
    let channel_id = channel.id();
    let channel_name = channel.name.clone();
    
    channel_cache
        .lock_arc()
        .await
        .insert(channel_id.clone(), channel)
        .await;

    // Broadcast channel create event to all connected clients
    let packet = QuicNetworkPacket {
        owner: Some(PacketOwner {
            name: String::from("channel_api"),
            client_id: vec![0u8; 0],
        }),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new_full(
            Create,
            user.clone(),
            channel_id.clone(),
            Some(channel_name),
            Some(user.clone()),
        )),
    };

    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send channel create packet to QUIC server: {}", e);
    }

    return status::Custom(Status::Ok, Some(Json(channel_id)));
}
