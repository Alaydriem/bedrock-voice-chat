use crate::http::guards::MCAccessToken;
use crate::http::openapi::{CustomJsonResponse, RouteSpec, TagDefinition};
use crate::services::ClientActionService;
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::control::ClientAction;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

inventory::submit! {
    TagDefinition {
        name: "Control",
        description: "In-game audio control plane. The mod submits ClientActions on behalf \
                      of the authenticated in-game player (X-MC-Access-Token).",
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

// Submits a ClientAction. Self/preference actions are delivered ClientBound to the
// actor's own connection; group actions mutate ChannelCollection. The actor is the
// body `id` (the mod attributes it from the in-game player); the token gates the
// route so players cannot call it. Returns the new nanoid for CreateGroup.
#[openapi(tag = "Control")]
#[post("/control", data = "<action>")]
pub async fn control(
    _access_token: MCAccessToken,
    cache_manager: &State<CacheManager>,
    webhook_receiver: &State<WebhookReceiver>,
    action: Json<ClientAction>,
) -> CustomJsonResponse<Option<String>> {
    let action = action.0;
    let svc = ClientActionService::new();

    if action.action.is_group_action() {
        let channels = cache_manager.get_channel_collection();
        let actor_cn = format!("minecraft:{}", action.id);
        match svc
            .route_group(&action.action, &actor_cn, &channels, webhook_receiver.inner())
            .await
        {
            Ok(created) => CustomJsonResponse::ok(created),
            Err(e) => {
                tracing::error!("route_group failed: {}", e);
                CustomJsonResponse::error(Status::InternalServerError)
            }
        }
    } else {
        match cache_manager.get_connection_registry() {
            Some(registry) => {
                svc.route_self(&action, &action.id, registry.as_ref());
                CustomJsonResponse::ok(None)
            }
            None => CustomJsonResponse::error(Status::ServiceUnavailable),
        }
    }
}
