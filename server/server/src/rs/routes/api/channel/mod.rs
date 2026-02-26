pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod event;

use crate::stream::quic::CacheManager;
use common::structs::channel::Channel;
use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

#[get("/?<id>")]
pub async fn channel_list<'r>(
    _identity: Certificate<'r>,
    cache_manager: &State<CacheManager>,
    id: Option<String>,
) -> status::Custom<Json<Vec<Channel>>> {
    let channel_collection = cache_manager.get_channel_collection();

    let channels: Vec<Channel> = match id {
        Some(ref id) => match channel_collection.get(id).await {
            Some(channel) => vec![channel],
            None => return status::Custom(Status::NotFound, Json(vec![])),
        },
        None => channel_collection.list(),
    };

    status::Custom(Status::Ok, Json(channels))
}
