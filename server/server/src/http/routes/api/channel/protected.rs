use crate::http::guards::PlayerGuard;
use crate::http::openapi::CustomJsonResponse;
use crate::services::{ChannelMembershipService, GroupKeyRelayService, MetricsService};
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::errors::ProtectRefusal;
use common::request::channel::ProtectedChannelCreateRequest;
use common::structs::channel::GroupName;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;
use std::sync::Arc;

/// Create an end-to-end encrypted channel and return its id.
///
/// The creator is not added: the app joins through `PUT /<id>/protected` once it holds the
/// group's keys. The server never receives any of the group's secrets.
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

/// Turn the caller's open channel into an end-to-end encrypted one.
///
/// Only the creator may, and only while nobody else is in it. The body names the refusal:
/// `not_found` (404), `not_creator` (403), `already_protected` or `not_alone` (409). A creator
/// whose app never opens the group's key socket loses the group after the creator deadline.
#[openapi(tag = "Channels")]
#[put("/<id>/protect")]
pub async fn channel_protect(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    relay: &State<Arc<GroupKeyRelayService>>,
    id: &str,
) -> CustomJsonResponse<String> {
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let caller = guard.player.game.membership_key(&gamertag);
    let channels = cache_manager.get_channel_collection();

    if let Err(refusal) =
        ChannelMembershipService::protect(&channels, webhook_receiver.inner(), &caller, id).await
    {
        let status = match refusal {
            ProtectRefusal::NotFound => Status::NotFound,
            ProtectRefusal::NotCreator => Status::Forbidden,
            ProtectRefusal::AlreadyProtected | ProtectRefusal::NotAlone => Status::Conflict,
        };
        return CustomJsonResponse::custom(status, Some(refusal.as_code().to_string()));
    }

    let webhook = webhook_receiver.inner().clone();
    let expired = id.to_string();
    relay.arm_creator_deadline(id.to_string(), move || {
        tokio::spawn(async move {
            ChannelMembershipService::close(&channels, &webhook, &caller, &expired).await;
        });
    });

    CustomJsonResponse::ok(id.to_string())
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
