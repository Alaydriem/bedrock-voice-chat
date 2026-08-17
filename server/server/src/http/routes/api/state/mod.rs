use crate::http::guards::GameAccessToken;
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

/// Return a player's cached self-state.
///
/// Used by the Addon to seed the in-game control panel.
#[openapi(tag = "Control")]
#[get("/state?<id>&<game>")]
pub async fn get_state(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    id: String,
    // Mirrors the `game` on the control request. Optional for the same reason: the
    // callers are hand-written encoders in two other languages.
    game: Option<Game>,
) -> CustomJsonResponse<Option<QueryState>> {
    // `id` arrives bare because the caller is a game mod, which knows a player by their
    // gamertag. Every cache behind this route is keyed on the canonical identity, so it is
    // composed here — the same shape `/api/position` uses.
    let identity = game.unwrap_or(Game::Minecraft).membership_key(&id);

    // Overlay current_group from server-authoritative membership rather than
    // trusting the client's reported value.
    let mut state = cache_manager.player_state().get(&identity).await;
    if let Some(ref mut s) = state {
        s.current_group = cache_manager
            .get_channel_collection()
            .get_player_channels(&identity)
            .into_iter()
            .next();
    }
    CustomJsonResponse::ok(state)
}

/// Return the owner's per-player preferences for a set of targets.
///
/// Scoped to the comma-separated `targets` the panel is showing, never the whole
/// preference store.
#[openapi(tag = "Control")]
#[get("/preferences?<owner>&<game>&<targets>")]
pub async fn get_preferences(
    _access_token: GameAccessToken,
    cache_manager: &State<CacheManager>,
    owner: String,
    // Mirrors the `game` on `/state`, for the same reason: `owner` arrives bare from a
    // game mod and the preference cache is keyed on the canonical identity.
    game: Option<Game>,
    targets: String,
) -> CustomJsonResponse<Vec<PlayerPreference>> {
    let targets: Vec<String> = if targets.is_empty() {
        Vec::new()
    } else {
        targets.split(',').map(str::to_string).collect()
    };
    let owner = game.unwrap_or(Game::Minecraft).membership_key(&owner);
    let prefs = cache_manager.preferences().get_scoped(&owner, &targets).await;
    CustomJsonResponse::ok(prefs)
}
