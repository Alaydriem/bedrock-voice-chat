use crate::rs::pool::AppDb;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channels::{
        ChannelEvent,
        ChannelEvents::{Join, Leave, Delete},
        ChannelPlayer,
    },
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use entity::player;
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use sea_orm_rocket::Connection as SeaOrmConnection;

#[put("/<id>", data = "<event>")]
pub async fn channel_event<'r>(
    identity: Certificate<'r>,
    db: SeaOrmConnection<'_, AppDb>,
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

    let channel = match cache_manager.get_channel(id).await {
        Some(channel) => channel,
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
            let conn = db.into_inner();
            let gamerpic = lookup_gamerpic(conn, &user, event.game.as_ref()).await;

            let channel_player = ChannelPlayer {
                name: user.clone(),
                game: event.game.clone(),
                gamerpic,
            };
            cache_manager.add_player_to_channel(channel_player, id).await;
        }
        Leave => {
            cache_manager.remove_player_from_channel(&user, id).await;
        }
        _ => {
            // Channel exists but unhandled event — just drop through to broadcast
            drop(channel);
        }
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

    return status::Custom(Status::Ok, Some(Json(true)));
}

async fn lookup_gamerpic(
    conn: &sea_orm::DatabaseConnection,
    gamertag: &str,
    game: Option<&common::Game>,
) -> Option<String> {
    use crate::services::GamerpicDecoder;

    let mut query = player::Entity::find()
        .filter(player::Column::Gamertag.eq(gamertag));

    if let Some(game) = game {
        query = query.filter(player::Column::Game.eq(game.clone()));
    }

    match query.one(conn).await {
        Ok(Some(record)) => GamerpicDecoder::decode(record.gamerpic),
        _ => None,
    }
}

async fn send_channel_event(packet: QuicNetworkPacket, webhook_receiver: &State<WebhookReceiver>) {
    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send packet to QUIC server: {}", e);
    }
}
