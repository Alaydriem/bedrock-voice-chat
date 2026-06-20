pub mod offer;
pub mod peer_link;
pub mod peer_redeem;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Relay",
        description: "Cross-server peering over direct peer HTTPS. Discovery is \
                      decentralized via in-realm announces (no central relay); these \
                      routes carry the offer / code-redeem / peer-link handshake \
                      between two servers that already share a realm.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api/relay",
        auto_mount: false,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                peer_link::peer_link
            ]
        },
    }
}
