pub mod code;
pub mod hytale;
pub mod introspect;
pub mod link_java;
pub mod minecraft;
pub mod state;

// Re-export HytaleSessionCache from dtos for route mounting
pub use crate::http::dtos::HytaleSessionCache;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Authentication",
        description: "OAuth2 authentication via Microsoft (Minecraft) or Hytale device code flow. \
                      Returns mTLS certificates and ncryptf keys for subsequent API calls. Interacting with these \
                      endpoints REQUIRES ncryptf. non-ncryptf request/response bodies are provided here as sample references.",
    }
}

inventory::submit! {
    TagDefinition {
        name: "Identity",
        description: "Player identity management. Retrieve current session state, permissions, \
                      and certificate renewal. Link external accounts such as Minecraft Java.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                minecraft::authenticate,
                hytale::start_device_flow,
                hytale::poll_status,
                code::code_authenticate,
                introspect::introspect,
                link_java::link_java_identity,
                state::auth_state
            ]
        },
    }
}
