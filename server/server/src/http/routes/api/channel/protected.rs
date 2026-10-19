use crate::http::guards::PlayerGuard;
use crate::http::openapi::CustomJsonResponse;
use crate::services::{ChannelMembershipService, GroupKeyRelayService, MetricsService};
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::request::channel::ProtectedChannelCreateRequest;
use common::structs::channel::GroupName;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;
use std::sync::Arc;

/// Create an end-to-end encrypted channel and return its id.
///
/// The creator is not added: the app joins through `PUT /<id>/protected` once it holds the
/// group's keys. The request carries no password, and the server never receives one.
#[openapi(tag = "Channels")]
#[post("/protected", data = "<request>")]
pub async fn channel_create_protected(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    relay: &State<Arc<GroupKeyRelayService>>,
    request: Json<ProtectedChannelCreateRequest>,
) -> CustomJsonResponse<String> {
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let creator = guard.player.game.membership_key(&gamertag);
    let channels = cache_manager.get_channel_collection();

    let name = if request.name.trim().is_empty() {
        let taken: Vec<String> = channels.list().into_iter().map(|c| c.name).collect();
        GroupName::next(&taken)
    } else {
        request.name.clone()
    };

    let id = ChannelMembershipService::create_protected(
        &channels,
        webhook_receiver.inner(),
        name,
        creator.clone(),
    )
    .await;

    // A creator who never opens a key socket leaves a group nobody can be admitted to.
    let webhook = webhook_receiver.inner().clone();
    let expired = id.clone();
    relay.arm_creator_deadline(id.clone(), move || {
        tokio::spawn(async move {
            ChannelMembershipService::close(&channels, &webhook, &creator, &expired).await;
        });
    });

    CustomJsonResponse::ok(id)
}

/// Join an end-to-end encrypted channel.
///
/// Membership here only makes the server route the group's ciphertext to this client. Being
/// admitted to the group's keys happens between members, over the group key socket.
#[openapi(tag = "Channels")]
#[put("/<id>/protected")]
pub async fn channel_join_protected(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    metrics: &State<Arc<MetricsService>>,
    id: &str,
) -> CustomJsonResponse<bool> {
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let user = guard.player.game.membership_key(&gamertag);
    let channels = cache_manager.get_channel_collection();

    let Some(channel) = channels.get(id).await else {
        return CustomJsonResponse::custom(Status::NotFound, Some(false));
    };
    if !channel.is_protected() {
        return CustomJsonResponse::custom(Status::Conflict, Some(false));
    }

    ChannelMembershipService::join(&channels, webhook_receiver.inner(), &user, id).await;
    metrics.record_channel_join();

    CustomJsonResponse::ok(true)
}
