pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod event;
pub(crate) mod rename;

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
use common::structs::channel::Channel;
use rocket::{http::Status, mtls::Certificate, State};
use crate::http::openapi::CustomJsonResponseRequired;
use rocket_okapi::openapi;

#[openapi(tag = "Channels")]
#[get("/?<id>")]
pub async fn channel_list(
    _identity: Certificate<'_>,
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
