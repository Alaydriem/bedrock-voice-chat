use crate::http::guards::MCAccessToken;
use crate::http::openapi::{CustomJsonResponse, RouteSpec};
use crate::stream::quic::{CacheManager, CacheTrait};
use common::Game;
use common::structs::control::{PlayerPreference, QueryState};
use rocket::State;
use rocket_okapi::openapi;

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: get_state, get_preferences]
        },
    }
}

// The mod reads a player's cached self-state to seed the in-game panel.
#[openapi(tag = "Control")]
#[get("/state?<id>")]
pub async fn get_state(
    _access_token: MCAccessToken,
    cache_manager: &State<CacheManager>,
    id: String,
) -> CustomJsonResponse<Option<QueryState>> {
    // Overlay current_group from server-authoritative membership rather than
    // trusting the client's reported value.
    let mut state = cache_manager.player_state().get(&id).await;
    if let Some(ref mut s) = state {
        let cn = Game::Minecraft.membership_key(&id);
        s.current_group = cache_manager
            .get_channel_collection()
            .get_player_channels(&cn)
            .into_iter()
            .next();
    }
    CustomJsonResponse::ok(state)
}

// The mod reads the owner's per-player preferences SCOPED to the players the panel
// is showing (comma-separated targets) — never the whole store.
#[openapi(tag = "Control")]
#[get("/preferences?<owner>&<targets>")]
pub async fn get_preferences(
    _access_token: MCAccessToken,
    cache_manager: &State<CacheManager>,
    owner: String,
    targets: String,
) -> CustomJsonResponse<Vec<PlayerPreference>> {
    let targets: Vec<String> = if targets.is_empty() {
        Vec::new()
    } else {
        targets.split(',').map(str::to_string).collect()
    };
    let prefs = cache_manager.preferences().get_scoped(&owner, &targets).await;
    CustomJsonResponse::ok(prefs)
}
