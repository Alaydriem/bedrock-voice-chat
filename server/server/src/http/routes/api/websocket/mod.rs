pub mod positions;
pub mod protocol_channel;
pub mod ticket;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "WebSocket",
        description: "Ticket exchange and streaming feeds for client UI surfaces. A browser \
                      cannot present a client certificate when opening a socket, so identity is \
                      established over mTLS here and spent once at upgrade time.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: ticket::ticket]
        },
    }
}
