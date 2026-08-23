use crate::http::openapi::CustomJsonResponse;
use crate::services::ChannelMembershipService;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use crate::http::guards::PlayerGuard;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

/// Create a channel and return its id.
///
/// Surfaced in the client as a group.
#[openapi(tag = "Channels")]
#[post("/", data = "<name>")]
pub async fn channel_create(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    name: Json<String>,
) -> CustomJsonResponse<String> {
    // Derived from the resolved player rather than read off the certificate CN, so a
    // legacy bare-gamertag certificate produces the same canonical identity as a current one.
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let user = guard.player.game.membership_key(&gamertag);

    let channel_id = ChannelMembershipService::create(
        &cache_manager.get_channel_collection(),
        webhook_receiver.inner(),
        name.0.clone(),
        user,
    )
    .await;

    CustomJsonResponse::ok(channel_id)
}
