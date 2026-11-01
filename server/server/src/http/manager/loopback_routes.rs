use rocket::{Route, routes};

use crate::http::openapi::OpenApiSpec;
use crate::http::routes;

/// The routes the game mods use, and nothing else.
///
/// The mods reach the server over plain HTTP from the same machine, so the listener that
/// serves them carries only what they call. Everything else, the player-facing API, the
/// admin routes, the metrics and the assets, stays on the TLS listener.
pub struct LoopbackRoutes;

impl LoopbackRoutes {
    /// URI paths the mods call. A route is mounted when its full path equals one of these or
    /// continues it at a segment boundary, so `/api/position` admits `/api/position/batch`
    /// and refuses `/api/positions`.
    pub const PATHS: [&'static str; 7] = [
        "/api/position",
        "/api/state",
        "/api/channel",
        "/api/control",
        "/api/preferences",
        "/api/audio/event",
        "/api/websocket/chat",
    ];

    /// `(mount prefix, routes)` pairs, shaped like `OpenApiSpec::routes()`.
    pub fn routes() -> Vec<(&'static str, Vec<Route>)> {
        let mut grouped = OpenApiSpec::routes();
        // Mounted directly rather than through the OpenAPI spec, for the same reason as on
        // the TLS listener: a WebSocket upgrade has no response schema.
        grouped.push(("/api", routes![routes::api::websocket::chat::chat]));

        grouped
            .into_iter()
            .map(|(prefix, routes)| {
                let kept = routes
                    .into_iter()
                    .filter(|route| Self::admits(&Self::full_path(prefix, route)))
                    .collect::<Vec<_>>();
                (prefix, kept)
            })
            .filter(|(_, routes)| !routes.is_empty())
            .collect()
    }

    /// Whether `path` is one of `PATHS` or continues one at a segment boundary.
    pub fn admits(path: &str) -> bool {
        Self::PATHS.iter().any(|allowed| {
            path == *allowed
                || path
                    .strip_prefix(allowed)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
    }

    /// The path a route answers at once mounted under `prefix`.
    pub fn full_path(prefix: &str, route: &Route) -> String {
        format!("{}{}", prefix.trim_end_matches('/'), route.uri.path())
    }
}
