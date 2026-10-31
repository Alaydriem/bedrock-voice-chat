use common::curia;
use crate::http::guards::GameAccessToken;
use crate::http::openapi::{CustomJsonResponse, RouteSpec, TagDefinition};
use crate::services::PlayerIdentityService;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::Game;
use common::structs::control::{GroupLinkAction, GroupLinkDrain, LinkedChannelView};
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

inventory::submit! {
    TagDefinition {
        name: "GroupLink",
        description: "Simple Voice Chat bridge group links. The bridge mirrors BVC groups into \
                      SVC and back (Authorization: Bearer, the mod token).",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![
                settings: group_link_action,
                group_link_events,
                group_link_channels
            ]
        },
    }
}

/// Apply a group link change for the Simple Voice Chat bridge.
///
/// Returns the new channel id for `CreateLinked`. A `Close` of a protected channel is refused.
#[openapi(tag = "GroupLink")]
#[post("/group-link", data = "<action>")]
pub async fn group_link_action(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    identity_service: &State<PlayerIdentityService>,
    action: Json<GroupLinkAction>,
) -> CustomJsonResponse<Option<String>> {
    let action = match action.0 {
        GroupLinkAction::CreateLinked {
            creator,
            name,
            members_only,
        } => GroupLinkAction::CreateLinked {
            creator: identity_service.resolve_name(&creator, &Game::Minecraft).await,
            name,
            members_only,
        },
        other => other,
    };

    match cache_manager
        .group_link_service(webhook_receiver.inner().clone())
        .apply(action)
        .await
    {
        Ok(created) => CustomJsonResponse::ok(created),
        Err(e) => {
            curia::info!("group link action refused", { error: e.to_string() });
            CustomJsonResponse::error(Status::Conflict)
        }
    }
}

/// BVC channel changes after sequence number `after`.
#[openapi(tag = "GroupLink")]
#[get("/group-link/events?<after>")]
pub async fn group_link_events(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    after: u64,
) -> CustomJsonResponse<GroupLinkDrain> {
    let service = cache_manager.group_link_service(webhook_receiver.inner().clone());
    CustomJsonResponse::ok(service.feed().drain(after))
}

/// Every non-protected BVC channel, for the bridge's resync.
#[openapi(tag = "GroupLink")]
#[get("/group-link/channels")]
pub async fn group_link_channels(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
) -> CustomJsonResponse<Vec<LinkedChannelView>> {
    let service = cache_manager.group_link_service(webhook_receiver.inner().clone());
    CustomJsonResponse::ok(service.channels())
}
