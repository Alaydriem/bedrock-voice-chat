use rocket::{ response::status, mtls::Certificate, http::Status, State, serde::json::Json };

use moka::future::Cache;
use std::sync::Arc;

use common::structs::channel::Channel;

/// Creates a new channel
#[post("/", data = "<name>")]
pub async fn channel_create<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>
    >,
    name: Json<String>
) -> status::Custom<Option<Json<String>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let channel = Channel::new(name.0, user);
    channel_cache.lock_arc().await.insert(channel.id(), channel.clone()).await;

    return status::Custom(Status::Ok, Some(Json(channel.id())));
}
