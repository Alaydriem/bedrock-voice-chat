use std::sync::Arc;

use common::{request::CodeLoginRequest, response::LoginResponse};
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::config::{Features, Permissions, Server};
use crate::http::dtos::ncryptf::JsonMessage;
use crate::http::openapi::NcryptfJsonResponse;
use crate::http::pool::Db;
use crate::services::{AuthService, CertificateService};

#[openapi(tag = "Authentication")]
#[post("/auth/code", data = "<payload>")]
pub async fn code_authenticate(
    db: Db<'_>,
    payload: Json<CodeLoginRequest>,
    config: &State<Server>,
    cert_service: &State<Arc<CertificateService>>,
    features: &State<Features>,
    perm_config: &State<Permissions>,
) -> NcryptfJsonResponse<LoginResponse> {
    let conn = db.into_inner();

    match AuthService::login_with_code(
        conn,
        &payload.0,
        config.inner(),
        cert_service.inner(),
        features.inner(),
        perm_config.defaults.clone(),
    )
    .await
    {
        Ok(response) => NcryptfJsonResponse::from_inner(JsonMessage::create(
            Status::Ok,
            Some(response),
            None,
            None,
        )),
        Err(e) => {
            tracing::error!("Code login failed: {}", e);
            NcryptfJsonResponse::from_inner(JsonMessage::create(e.to_status(), None, None, None))
        }
    }
}
