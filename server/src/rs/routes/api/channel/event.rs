use common::structs::{
    channel::{ ChannelEvent, ChannelEvents::{ Join, Leave } },
    packet::{ ChannelEventPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData },
};
use rocket::{ response::status, mtls::Certificate, http::Status, State, serde::json::Json };

use moka::future::Cache;
use std::sync::Arc;

#[put("/<id>", data = "<event>")]
pub async fn channel_event<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>
    >,
    id: &str,
    queue: &State<Arc<deadqueue::limited::Queue<QuicNetworkPacket>>>,
    event: Json<ChannelEvent>
) -> status::Custom<Option<Json<bool>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let event = event.0;

    let lock = channel_cache.lock_arc().await;
    let mut channel = match lock.get(id).await {
        Some(channel) => channel,
        None => {
            return status::Custom(Status::BadRequest, Some(Json(false)));
        }
    };

    match event.event {
        Join => {
            _ = channel.add_player(user.clone());
        }
        Leave => {
            _ = channel.remove_player(user.clone());
        }
    }

    _ = lock.insert(id.to_string(), channel).await;
    drop(lock);

    let packet = QuicNetworkPacket {
        client_id: vec![0u8; 0],
        author: user.clone(),
        packet_type: PacketType::ChannelEvent,
        in_group: None,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket {
            event: event.event,
            name: user,
            channel: id.to_string(),
        }),
    };

    _ = queue.push(packet).await;

    return status::Custom(Status::Ok, Some(Json(true)));
}
