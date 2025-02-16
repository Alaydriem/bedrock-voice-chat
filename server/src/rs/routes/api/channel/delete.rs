use rocket::{http::Status, mtls::Certificate, response::status, serde::json::Json, State};

use moka::future::Cache;
use std::sync::Arc;

/// Deletes a channel if the identity provided with the certificate is the creator
#[delete("/<id>")]
pub async fn channel_delete<'r>(
    identity: Certificate<'r>,
    channel_cache: &State<
        Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
    >,
    id: String,
) -> status::Custom<Option<Json<bool>>> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return status::Custom(Status::Forbidden, None);
        }
    };

    let lock = channel_cache.lock_arc().await;
    match lock.get(&id).await {
        Some(channel) =>
        // Only allow the channel to be deleted by it's creator
        {
            match channel.creator.eq(&user) {
                true => {
                    lock.remove(&id).await;
                    return status::Custom(Status::Ok, Some(Json(true)));
                }
                false => {
                    return status::Custom(Status::Unauthorized, Some(Json(true)));
                }
            }
        }
        None => {
            return status::Custom(Status::NotFound, Some(Json(true)));
        }
    };
}
