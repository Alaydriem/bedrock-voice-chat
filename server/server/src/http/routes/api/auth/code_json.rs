use std::sync::Arc;

use common::{request::CodeLoginRequest, response::LoginResponse};
use rocket::{http::Status, serde::json::Json, State};
use rocket_okapi::openapi;

use crate::config::{Features, Permissions, Server};
use crate::http::pool::Db;
use crate::services::{AuthService, CertificateService};

#[openapi(tag = "Authentication")]
#[post("/auth/code/json", data = "<payload>")]
pub async fn code_authenticate_json(
    db: Db<'_>,
    payload: Json<CodeLoginRequest>,
    config: &State<Server>,
    cert_service: &State<Arc<CertificateService>>,
    features: &State<Features>,
    perm_config: &State<Permissions>,
) -> Result<Json<LoginResponse>, Status> {
    let conn = db.into_inner();

    AuthService::login_with_code(
        conn,
        &payload.0,
        config.inner(),
        cert_service.inner(),
        features.inner(),
        perm_config.defaults.clone(),
    )
    .await
    .map(Json)
    .map_err(|e| {
        tracing::error!("code_authenticate_json: {}", e);
        e.to_status()
    })
}
