use rocket::{State, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::GameAccessToken;
use crate::http::openapi::{RouteSpec, TagDefinition};
use crate::stream::quic::CacheManager;

inventory::submit! {
    TagDefinition {
        name: "Clients",
        description: "Which players currently hold a voice connection, for a mod that \
                      delivers audio of its own alongside this server's.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: live_clients]
        },
    }
}

/// The identities with a live voice connection to this server.
///
/// Asked for by a mod that also delivers audio — the Simple Voice Chat bridge — so it
/// can leave those players out of its own injection. A Java player running both SVC
/// and the BVC desktop client would otherwise hear every remote speaker twice.
///
/// Empty is a real answer, not a failure: a server whose players have not opened a
/// voice client suppresses nobody.
#[openapi(tag = "Clients")]
#[get("/clients/live")]
pub async fn live_clients(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
) -> Json<Vec<String>> {
    let Some(registry) = cache_manager.get_connection_registry() else {
        return Json(Vec::new());
    };

    // Sorted so a caller diffing two responses sees a change in membership rather
    // than a change in iteration order.
    let mut identities: Vec<String> = registry.on_voice_identities().into_iter().collect();
    identities.sort();

    Json(identities)
}
