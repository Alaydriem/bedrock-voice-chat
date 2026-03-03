pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod event;

use crate::rs::pool::AppDb;
use entity::player;
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use sea_orm_rocket::Connection as SeaOrmConnection;

use moka::future::Cache;
use std::sync::Arc;

use common::structs::channel::Channel;

#[get("/?<id>")]
pub async fn channel_list<'r>(
    _identity: Certificate<'r>,
    db: SeaOrmConnection<'_, AppDb>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
    >,
    id: Option<String>,
) -> status::Custom<Json<Vec<Channel>>> {
    let mut channels: Vec<Channel> = Vec::new();
    for (i, channel) in channel_cache.lock_arc().await.clone().iter() {
        match id.clone() {
            Some(id) => match id.eq(&i.to_string()) {
                true => channels.push(channel),
                false => {
                    continue;
                }
            },
            None => channels.push(channel),
        }
    }

    if id.is_some() && channels.len() == 0 {
        return status::Custom(Status::NotFound, Json(channels));
    }

    let conn = db.into_inner();
    enrich_channel_gamerpics(&mut channels, conn).await;

    return status::Custom(Status::Ok, Json(channels));
}

pub(crate) async fn enrich_channel_gamerpics(
    channels: &mut [Channel],
    conn: &sea_orm::DatabaseConnection,
) {
    let mut all_gamertags: Vec<String> = Vec::new();
    for channel in channels.iter() {
        for p in &channel.players {
            if !all_gamertags.contains(&p.name) {
                all_gamertags.push(p.name.clone());
            }
        }
    }

    if all_gamertags.is_empty() {
        return;
    }

    let players = match player::Entity::find()
        .filter(player::Column::Gamertag.is_in(all_gamertags))
        .all(conn)
        .await
    {
        Ok(players) => players,
        Err(e) => {
            tracing::error!("Failed to enrich channel gamerpics: {}", e);
            return;
        }
    };

    for channel in channels.iter_mut() {
        for channel_player in &mut channel.players {
            if channel_player.gamerpic.is_some() {
                continue;
            }

            // Find matching DB record, preferring exact game match
            let matching = players.iter().find(|db_player| {
                db_player.gamertag.as_deref() == Some(&channel_player.name)
                    && (channel_player.game.is_none()
                        || channel_player.game.as_ref() == Some(&db_player.game))
            });

            // Fall back to gamertag-only match
            let matching = matching.or_else(|| {
                players.iter().find(|db_player| {
                    db_player.gamertag.as_deref() == Some(&channel_player.name)
                        && db_player.gamerpic.is_some()
                })
            });

            if let Some(db_player) = matching {
                channel_player.gamerpic = db_player.gamerpic.clone();
                if channel_player.game.is_none() {
                    channel_player.game = Some(db_player.game.clone());
                }
            }
        }
    }
}
