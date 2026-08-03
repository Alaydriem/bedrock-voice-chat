use std::sync::Arc;

use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_okapi::openapi;
use sea_orm::DatabaseConnection;

use crate::http::dtos::health::ReadinessResponse;
use crate::http::pool::Db;
use crate::services::HealthService;

/// Readiness probe. Reports that the server can accept traffic.
///
/// Unauthenticated. Gate a load balancer on this rather than on liveness.
#[openapi(tag = "Health")]
#[get("/readiness")]
pub async fn readiness(
    db: Db<'_>,
    health: &State<Arc<HealthService>>,
) -> (Status, Json<ReadinessResponse>) {
    let conn: &DatabaseConnection = db.into_inner();
    let response = health.evaluate(conn).await;
    let status = if response.ready() {
        Status::Ok
    } else {
        Status::ServiceUnavailable
    };
    (status, Json(response))
}
