pub(crate) mod liveness;
pub(crate) mod ping;
pub(crate) mod readiness;

pub use ping::pong;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Health",
        description: "Health and readiness probes. Liveness and readiness are unauthenticated \
                      for use with Kubernetes.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: ping::pong]
        },
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/health",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                liveness::liveness,
                readiness::readiness
            ]
        },
    }
}
