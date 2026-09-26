pub mod create;
pub mod delete;
pub mod event;
pub mod rename;

use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::stream::quic::CacheManager;

inventory::submit! {
    TagDefinition {
        name: "Channels",
        description: "Voice channel management. Create, delete, rename, join, and leave \
                      channels. Channel membership is broadcast to connected QUIC clients.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api/channel",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                create::channel_create,
                delete::channel_delete,
                event::channel_event,
                channel_list,
                rename::channel_rename
            ]
        },
    }
}
use crate::http::guards::ChannelReader;
use crate::http::openapi::CustomJsonResponseRequired;
use common::structs::channel::Channel;
use rocket::{State, http::Status};
use rocket_okapi::openapi;

/// List channels on the server.
///
/// Readable by a player certificate or a game server access token: the in-game panel's
/// group page is a game server asking on a player's behalf.
#[openapi(tag = "Channels")]
#[get("/?<id>")]
pub async fn channel_list(
    _guard: ChannelReader,
    cache_manager: &State<CacheManager>,
    id: Option<String>,
) -> CustomJsonResponseRequired<Vec<Channel>> {
    let channel_collection = cache_manager.get_channel_collection();

    let channels: Vec<Channel> = match id {
        Some(ref id) => match channel_collection.get(id).await {
            Some(channel) => vec![channel],
            None => return CustomJsonResponseRequired::custom(Status::NotFound, vec![]),
        },
        None => channel_collection.list(),
    };

    CustomJsonResponseRequired::ok(channels)
}
