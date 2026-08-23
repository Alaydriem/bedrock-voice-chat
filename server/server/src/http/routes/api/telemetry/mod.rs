use common::curia;
use std::sync::Arc;

use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::GameAccessToken;
use crate::http::openapi::{CustomJsonResponse, RouteSpec, TagDefinition};
use crate::services::{HostCapability, MetricsService};

inventory::submit! {
    TagDefinition {
        name: "Telemetry",
        description: "Facts a Java mod observes about its own host, which the mod has \
                      no channel of its own to report.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings: host_capability]
        },
    }
}

/// Report whether this Minecraft host could fetch and write a native library.
///
/// Answers one question: what share of hosts could run the skinny mod jar, which
/// resolves its native library at runtime instead of carrying every platform. Both
/// jars report it, so the two populations are comparable.
///
/// The body carries no hostname, address, path, or player data — only the jar
/// variant, the platform, the mod version and the two outcomes. Anything outside
/// the known vocabulary is refused rather than forwarded, because this arrives from
/// a third-party jar and would otherwise put unbounded strings into the metrics
/// pipeline.
#[openapi(tag = "Telemetry")]
#[post("/telemetry/host-capability", data = "<report>")]
pub async fn host_capability(
    _access_token: GameAccessToken,
    metrics: &State<Arc<MetricsService>>,
    report: Json<serde_json::Value>,
) -> CustomJsonResponse<Option<String>> {
    let parsed = match HostCapability::parse(&report.0.to_string()) {
        Ok(parsed) => parsed,
        Err(reason) => {
            curia::info!(format!("refusing a host capability report: {reason}"));
            return CustomJsonResponse::error(Status::BadRequest);
        }
    };

    metrics.record_host_capability(parsed);
    CustomJsonResponse::ok(None)
}
