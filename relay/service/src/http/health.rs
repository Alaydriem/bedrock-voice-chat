use axum::http::StatusCode;

pub struct HealthRoutes;

impl HealthRoutes {
    pub async fn healthz() -> StatusCode {
        StatusCode::OK
    }
}
