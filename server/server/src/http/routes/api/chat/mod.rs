use std::sync::Arc;

use common::structs::chat::{ChatMode, ChatWorld};
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::http::pool::Db;
use crate::services::{AuthService, BedrockEventService, ChatService};

inventory::submit! {
    TagDefinition {
        name: "Chat",
        description: "In-game chat relay. Lists the worlds a player has been seen in and which \
                      of them currently have a chat channel.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: worlds]
        },
    }
}

/// Worlds this player has been seen in, newest first.
///
/// `active` and `available` are independent, and the composer has to tell them apart: a world
/// can be hosting players with its chat channel down, which reads very differently from a
/// world nobody is in.
#[openapi(tag = "Chat")]
#[get("/chat/worlds")]
pub async fn worlds(
    cert: Certificate<'_>,
    db: Db<'_>,
    chat_service: &State<Arc<ChatService>>,
    bedrock_event_service: &State<Arc<BedrockEventService>>,
) -> Result<Json<Vec<ChatWorld>>, Status> {
    let conn = db.into_inner();
    let player = AuthService::player_from_certificate(&cert, conn, None).await?;

    let rows = entity::player_world::Entity::find()
        .filter(entity::player_world::Column::PlayerId.eq(player.id))
        .order_by_desc(entity::player_world::Column::LastSeen)
        .all(conn)
        .await
        .map_err(|e| {
            tracing::error!("failed to read chat world history: {}", e);
            Status::InternalServerError
        })?;

    // The world the player is standing in right now.
    //
    // History alone is not enough: a row is written when somebody speaks, so a player who has
    // never typed in this world would never be offered it — and so could never type here.
    // Presence is what breaks that deadlock, and it also keeps the list right on a first join.
    let identity = player
        .gamertag
        .as_ref()
        .map(|tag| player.game.membership_key(tag));

    let live_world = match identity.as_deref() {
        Some(id) => chat_service.live_world_of(id).await,
        None => None,
    };

    if let (Some(id), Some(world)) = (identity.as_deref(), live_world.as_deref()) {
        // Persisted so the picker still offers it once they are out of game. Throttled inside
        // the service, so calling it on every poll is cheap.
        chat_service.note_presence(id, world).await;
    }

    let mut out = Vec::with_capacity(rows.len() + 1);

    // Listed first when it is not already in history: it is where they are.
    if let Some(world) = live_world.as_deref() {
        if !rows.iter().any(|r| r.world_uuid == world) && chat_service.is_available(world) {
            let active = bedrock_event_service.is_bds_healthy(world).await;
            out.push(ChatWorld {
                available: true,
                mode: if active { ChatMode::Server } else { ChatMode::Local },
                world_name: chat_service
                    .world_name(world)
                    .unwrap_or_else(|| world.to_string()),
                world_uuid: world.to_string(),
                last_seen: common::ncryptflib::rocket::Utc::now().timestamp().max(0) as u64,
                active,
            });
        }
    }

    for row in rows {
        let active = bedrock_event_service.is_bds_healthy(&row.world_uuid).await;
        out.push(ChatWorld {
            available: chat_service.is_available(&row.world_uuid),
            // A mod reporting positions means the addon owns chat there; otherwise the
            // client's own proxy is the only source.
            mode: if active {
                ChatMode::Server
            } else {
                ChatMode::Local
            },
            // The live name from `hello` wins: a world can be renamed between sessions.
            world_name: chat_service
                .world_name(&row.world_uuid)
                .unwrap_or(row.world_name),
            world_uuid: row.world_uuid,
            last_seen: row.last_seen.max(0) as u64,
            active,
        });
    }

    Ok(Json(out))
}
