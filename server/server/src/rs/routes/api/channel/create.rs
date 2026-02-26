use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::{Channel, ChannelEvents::Create},
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

#[post("/", data = "<name>")]
pub async fn channel_create<'r>(
    identity: Certificate<'r>,
    cache_manager: &State<CacheManager>,
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

    cache_manager
        .get_channel_collection()
        .insert(channel.clone())
        .await;

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

    status::Custom(Status::Ok, Some(Json(channel_id)))
}
