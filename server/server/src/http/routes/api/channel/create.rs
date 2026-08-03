use crate::http::openapi::CustomJsonResponse;
use crate::services::ChannelMembershipService;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;

/// Create a channel and return its id.
///
/// Surfaced in the client as a group.
#[openapi(tag = "Channels")]
#[post("/", data = "<name>")]
pub async fn channel_create(
    identity: Certificate<'_>,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    name: Json<String>,
) -> CustomJsonResponse<String> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return CustomJsonResponse::error(Status::Forbidden);
        }
    };

    let channel_id = ChannelMembershipService::create(
        &cache_manager.get_channel_collection(),
        webhook_receiver.inner(),
        name.0.clone(),
        user,
    )
    .await;

    CustomJsonResponse::ok(channel_id)
}
