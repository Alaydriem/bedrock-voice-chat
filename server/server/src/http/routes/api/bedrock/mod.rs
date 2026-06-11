mod transfer;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Bedrock",
        description: "Bedrock protocol transfer relay endpoints",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: transfer::register_transfer_target]
        },
    }
}
