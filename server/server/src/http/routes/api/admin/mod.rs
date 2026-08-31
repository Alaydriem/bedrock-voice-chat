pub mod permission;
pub mod relay;
pub mod user;

use rocket::http::Status;

use crate::http::openapi::{RouteSpec, TagDefinition};

/// Validates a gamertag string for basic constraints.
pub fn validate_gamertag(gamertag: &str) -> Result<(), Status> {
    let trimmed = gamertag.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(Status::BadRequest);
    }
    Ok(())
}

inventory::submit! {
    TagDefinition {
        name: "Admin",
        description: "Operator-only administrative endpoints. Caller must present a valid mTLS \
                      certificate AND hold the `admin` permission.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api/admin",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![
                settings:
                    user::create::create_user,
                    user::list::list_users,
                    user::banish::banish_user,
                    user::generate_code::generate_code,
                    permission::set::set_permission,
                    permission::clear::clear_permission,
                    permission::list::list_permissions,
                    relay::peerlink::relay_peerlink,
                    relay::worlds::relay_worlds,
                    relay::pair::relay_pair,
                    relay::pair::relay_paired,
                    relay::pair::relay_unpair
            ]
        },
    }
}
