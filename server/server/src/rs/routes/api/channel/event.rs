use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::{
        ChannelEvent,
        ChannelEvents::{Delete, Join, Leave},
    },
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

#[put("/<id>", data = "<event>")]
pub async fn channel_event<'r>(
    identity: Certificate<'r>,
    cache_manager: &State<CacheManager>,
    id: &str,
    webhook_receiver: &State<WebhookReceiver>,
    event: Json<ChannelEvent>,
) -> status::Custom<Option<Json<bool>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let event = event.0;
    let channel_collection = cache_manager.get_channel_collection();

    match channel_collection.get(id).await {
        Some(_) => {}
        None => {
            if event.event.eq(&Delete) {
                let packet = QuicNetworkPacket {
                    owner: Some(PacketOwner {
                        name: String::from("channel_api"),
                        client_id: vec![0u8; 0],
                    }),
                    packet_type: PacketType::ChannelEvent,
                    data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
                        event.event,
                        user,
                        id.to_string(),
                    )),
                };

                send_channel_event(packet, webhook_receiver).await;
                return status::Custom(Status::Ok, Some(Json(true)));
            } else {
                return status::Custom(Status::BadRequest, Some(Json(false)));
            }
        }
    };

    match event.event {
        Join => {
            channel_collection
                .add_player_to_channel(&user, id)
                .await;
        }
        Leave => {
            channel_collection
                .remove_player_from_channel(&user, id)
                .await;
        }
        _ => {}
    }

    let packet = QuicNetworkPacket {
        owner: Some(PacketOwner {
            name: String::from("channel_api"),
            client_id: vec![0u8; 0],
        }),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
            event.event,
            user,
            id.to_string(),
        )),
    };

    send_channel_event(packet, webhook_receiver).await;

    status::Custom(Status::Ok, Some(Json(true)))
}

async fn send_channel_event(packet: QuicNetworkPacket, webhook_receiver: &State<WebhookReceiver>) {
    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send packet to QUIC server: {}", e);
    }
}
