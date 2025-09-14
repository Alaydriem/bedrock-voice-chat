pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod event;

use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

use moka::future::Cache;
use std::sync::Arc;

use common::structs::channel::Channel;

#[get("/?<id>")]
pub async fn channel_list<'r>(
    _identity: Certificate<'r>,
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

    return status::Custom(Status::Ok, Json(channels));
}
