use common::structs::channel::{ ChannelEvents::{ Join, Leave }, ChannelEvent };
use rocket::{ response::status, mtls::Certificate, http::Status, State, serde::json::Json };

use moka::future::Cache;
use std::sync::Arc;

#[put("/<id>", data = "<event>")]
pub async fn channel_event<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>
    >,
    id: String,
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
    let mut channel = match lock.get(&id).await {
        Some(channel) => channel,
        None => {
            return status::Custom(Status::BadRequest, Some(Json(false)));
        }
    };

    match event.event {
        Join => {
            _ = channel.add_player(user);
        }
        Leave => {
            _ = channel.remove_player(user);
        }
    }

    println!("{:?}", &channel);

    _ = lock.insert(id, channel).await;
    drop(lock);

    return status::Custom(Status::Ok, Some(Json(true)));
}
