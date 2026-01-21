use std::time::Instant;

use common::{
    auth::{HytaleAuthProvider, PollResult},
    ncryptflib as ncryptf,
    structs::config::{
        HytaleAuthStatus, HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse,
    },
    Game,
};
use rocket::{http::Status, State};

use sea_orm_rocket::Connection as SeaOrmConnection;

use crate::config::ApplicationConfigServer;
use crate::rs::pool::AppDb;
use crate::rs::structs::ncryptf_json::JsonMessage;
use crate::rs::structs::{build_login_response, AuthError, HytaleSession, HytaleSessionCache};

/// Start a new Hytale device code flow
/// Returns session_id and user code for the client to display
#[post("/auth/hytale/start-device-flow")]
pub async fn start_device_flow(
    session_cache: &State<HytaleSessionCache>,
) -> ncryptf::rocket::JsonResponse<JsonMessage<HytaleDeviceFlowStartResponse>> {
    // Create the Hytale auth provider and start the device flow
    let provider = HytaleAuthProvider::new();

    let flow = match provider.start_device_flow().await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to start Hytale device flow: {}", e);
            return JsonMessage::create(Status::InternalServerError, None, None, None);
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

    JsonMessage::create(Status::Ok, Some(response), None, None)
}

/// Poll the status of a Hytale device code flow
#[get("/auth/hytale/status/<session_id>")]
pub async fn poll_status(
    db: SeaOrmConnection<'_, AppDb>,
    config: &State<ApplicationConfigServer>,
    session_cache: &State<HytaleSessionCache>,
    session_id: &str,
) -> ncryptf::rocket::JsonResponse<JsonMessage<HytaleDeviceFlowStatusResponse>> {
    let conn = db.into_inner();

    // Look up the session
    let session = match session_cache.get(session_id).await {
        Some(s) => s,
        None => {
            tracing::warn!("Hytale session not found: {}", session_id);
            return JsonMessage::create(Status::NotFound, None, None, None);
        }
    };

    // Check if session has expired
    if session.expires_at < Instant::now() {
        session_cache.remove(session_id).await;
        let response = HytaleDeviceFlowStatusResponse {
            status: HytaleAuthStatus::Expired,
            login_response: None,
        };
        return JsonMessage::create(Status::Ok, Some(response), None, None);
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
            return JsonMessage::create(Status::Ok, Some(response), None, None);
        }
    };

    match poll_result {
        PollResult::Pending | PollResult::SlowDown => {
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Pending,
                login_response: None,
            };
            JsonMessage::create(Status::Ok, Some(response), None, None)
        }
        PollResult::Expired => {
            session_cache.remove(session_id).await;
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Expired,
                login_response: None,
            };
            JsonMessage::create(Status::Ok, Some(response), None, None)
        }
        PollResult::Denied => {
            session_cache.remove(session_id).await;
            let response = HytaleDeviceFlowStatusResponse {
                status: HytaleAuthStatus::Denied,
                login_response: None,
            };
            JsonMessage::create(Status::Ok, Some(response), None, None)
        }
        PollResult::Success(auth_result) => {
            // Remove session from cache (single use)
            session_cache.remove(session_id).await;

            // Build login response using shared logic
            // AuthResult already contains gamertag and gamerpic
            match build_login_response(
                conn,
                config.inner(),
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
                    JsonMessage::create(Status::Ok, Some(response), None, None)
                }
                Err(e) => {
                    tracing::error!("Login with hytale failed: {}", e);
                    match e {
                        AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                            JsonMessage::create(Status::Forbidden, None, None, None)
                        }
                        _ => {
                            let response = HytaleDeviceFlowStatusResponse {
                                status: HytaleAuthStatus::Error,
                                login_response: None,
                            };
                            JsonMessage::create(Status::Ok, Some(response), None, None)
                        }
                    }
                }
            }
        }
    }
}
