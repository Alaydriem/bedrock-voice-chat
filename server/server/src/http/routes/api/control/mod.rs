use common::curia;
use crate::config::Voice;
use crate::http::guards::GameAccessToken;
use crate::http::openapi::{CustomJsonResponse, RouteSpec, TagDefinition};
use crate::services::{ClientActionService, PlayerIdentityService};
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::Game;
use common::structs::control::ClientAction;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

inventory::submit! {
    TagDefinition {
        name: "Control",
        description: "In-game audio control plane. The mod submits ClientActions on behalf \
                      of the authenticated in-game player (Authorization: Bearer).",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: control]
        },
    }
}

/// Submit a client action on behalf of an in-game player.
///
/// Self and preference actions are delivered ClientBound to the actor's own
/// connection; group actions mutate the channel collection. The actor is the body
/// `id`, which the mod attributes from the in-game player, and the access token
/// gates the route so players cannot call it directly.
///
/// Returns the new nanoid for CreateGroup.
#[openapi(tag = "Control")]
#[post("/control", data = "<action>")]
pub async fn control(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    identity_service: &State<PlayerIdentityService>,
    voice: &State<Voice>,
    action: Json<ClientAction>,
) -> CustomJsonResponse<Option<String>> {
    let mut action = action.0;
    // Resolve the in-game name to its canonical gamertag (Floodgate/Java aliases)
    // before routing, matching the position ingress so control actions key on the
    // same identity the voice plane uses. The alias table is per-game, so this uses
    // the game the request declared rather than assuming one.
    let game = action.game.clone().unwrap_or(Game::Minecraft);
    action.id = identity_service.resolve_name(&action.id, &game).await;

    let svc = ClientActionService::new(voice.recording.enabled);

    if !svc.permits(&action.action) {
        curia::info!("control refused: this server does not permit recording");
        return CustomJsonResponse::error(Status::Forbidden);
    }

    if action.action.is_group_action() {
        let channels = cache_manager.get_channel_collection();
        let actor = action.actor_identity();
        match ClientActionService::route_group(
            &action.action,
            &actor,
            &channels,
            webhook_receiver.inner(),
        )
        .await
        {
            Ok(created) => CustomJsonResponse::ok(created),
            Err(e) => {
                // route_group only errors on a JoinGroup miss (unknown share code) —
                // a client error, not a server fault.
                curia::info!("route_group rejected: {}", e);
                CustomJsonResponse::error(Status::NotFound)
            }
        }
    } else {
        match cache_manager.get_connection_registry() {
            Some(registry) => {
                svc.route_self_with_echo(
                    &action,
                    &action.actor_key(),
                    registry.as_ref(),
                    cache_manager.player_state(),
                    cache_manager.preferences(),
                )
                .await;
                CustomJsonResponse::ok(None)
            }
            None => CustomJsonResponse::error(Status::ServiceUnavailable),
        }
    }
}
