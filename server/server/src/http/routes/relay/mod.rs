pub mod challenge;
pub mod lookup;
pub mod peer_cert;
pub mod proof;
pub mod register;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Relay",
        description: "Cross-server peering relay. Discovery routes (challenge, register, lookup) \
                      are mounted when the relay feature is enabled; the proof and peer-cert \
                      endpoints are mounted when this server runs the relay client. All are \
                      authenticated out-of-band via SPKI pinning and endpoint-control proofs.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/relay",
        auto_mount: false,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                challenge::challenge,
                register::register,
                lookup::lookup,
                proof::proof,
                peer_cert::peer_cert
            ]
        },
    }
}
