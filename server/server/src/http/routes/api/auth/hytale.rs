use std::time::Instant;

use common::{
    auth::{HytaleAuthProvider, PollResult},
    structs::config::{
        HytaleAuthStatus, HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse,
    },
    Game,
};
use rocket::{http::Status, State};
use rocket_okapi::openapi;

use crate::config::{Permissions, Server};
use crate::http::dtos::ncryptf::JsonMessage;
use crate::http::dtos::{HytaleSession, HytaleSessionCache};
use crate::http::guards::HytaleSessionId;
use crate::http::openapi::NcryptfJsonResponse;
use crate::http::pool::Db;
use crate::services::{AuthError, AuthService, PermissionService};

/// Start a new Hytale device code flow
/// Returns session_id and user code for the client to display
#[openapi(tag = "Authentication")]
#[post("/auth/hytale/start-device-flow")]
pub async fn start_device_flow(
    session_cache: &State<HytaleSessionCache>,
) -> NcryptfJsonResponse<HytaleDeviceFlowStartResponse> {
    // Create the Hytale auth provider and start the device flow
    let provider = HytaleAuthProvider::new();

    let flow = match provider.start_device_flow().await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to start Hytale device flow: {}", e);
            return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::InternalServerError, None, None, None));
        }
    };

    // Generate a unique session ID
    let session_id = nanoid::nanoid!(32);

    // Build the response from the flow data (convert u64 to u32 for DTO)
    let response = HytaleDeviceFlowStartResponse {
        session_id: session_id.clone(),
        user_code: flow.user_code.clone(),
        verification_uri: flow.verification_uri.clone(),
        verification_uri_complete: flow.verification_uri_complete.clone(),
        expires_in: flow.expires_in as u32,
        interval: flow.interval as u32,
    };

    // Store the session with the flow (use flow.expires_in which is u64)
    let expires_in = flow.expires_in;
    let session = HytaleSession {
        flow,
        expires_at: Instant::now() + std::time::Duration::from_secs(expires_in),
    };

    session_cache.insert(session_id, session).await;

    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
}

/// Poll the status of a Hytale device code flow
#[openapi(tag = "Authentication")]
#[get("/auth/hytale/status")]
pub async fn poll_status(
    db: Db<'_>,
    config: &State<Server>,
    perm_config: &State<Permissions>,
    session_cache: &State<HytaleSessionCache>,
    session_id: HytaleSessionId,
) -> NcryptfJsonResponse<HytaleDeviceFlowStatusResponse> {
    let conn = db.into_inner();
    let session_id = &session_id.0;

    // Look up the session
    let session = match session_cache.get(session_id).await {
        Some(s) => s,
        None => {
            tracing::warn!("Hytale session not found: {}", session_id);
            return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::NotFound, None, None, None));
        }
    };

    // Check if session has expired
    if session.expires_at < Instant::now() {
        session_cache.remove(session_id).await;
        let response = HytaleDeviceFlowStatusResponse {
            status: HytaleAuthStatus::Expired,
            login_response: None,
        };
        return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None));
    }

    // Create provider and poll for completion
    let provider = HytaleAuthProvider::new();

    let poll_result = match provider.poll(&session.flow).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to poll Hytale token: {}", e);
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Error,
                login_response: None,
            };
            return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None));
        }
    };

    match poll_result {
        PollResult::Pending | PollResult::SlowDown => {
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Pending,
                login_response: None,
            };
            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
        }
        PollResult::Expired => {
            session_cache.remove(session_id).await;
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Expired,
                login_response: None,
            };
            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
        }
        PollResult::Denied => {
            session_cache.remove(session_id).await;
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Denied,
                login_response: None,
            };
            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
        }
        PollResult::Success(auth_result) => {
            // Remove session from cache (single use)
            session_cache.remove(session_id).await;

            // Build login response using AuthService
            // AuthResult already contains gamertag and gamerpic
            let perm_service = PermissionService::new(perm_config.defaults.clone());
            match AuthService::build_login_response(
                conn,
                config.inner(),
                Some(&perm_service),
                auth_result.gamertag,
                auth_result.gamerpic,
                Game::Hytale,
            )
            .await
            {
                Ok(login_response) => {
                    let response = HytaleDeviceFlowStatusResponse {
                        status: HytaleAuthStatus::Success,
                        login_response: Some(login_response),
                    };
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
                }
                Err(e) => {
                    tracing::error!("Login with hytale failed: {}", e);
                    match e {
                        AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None))
                        }
                        _ => {
                            let response = HytaleDeviceFlowStatusResponse {
                                status: HytaleAuthStatus::Error,
                                login_response: None,
                            };
                            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
                        }
                    }
                }
            }
        }
    }
}
