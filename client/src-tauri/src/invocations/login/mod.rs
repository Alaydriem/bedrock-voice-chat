use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::invocations::get_reqwest_client;
const CONFIG_ENDPOINT: &'static str = "/api/config";

/// Checks the API and ensures that we can connect to it.
#[tauri::command(async)]
pub(crate) async fn check_api_status(server: String) -> Result<bool, bool> {
    let client = get_reqwest_client();
    let endpoint = format!(
        "https://{}/{}",
        server.replace("https://", ""),
        CONFIG_ENDPOINT
    );

    match client.get(endpoint).send().await {
        Ok(response) => match response.status() {
            StatusCode::OK => match response.json::<common::structs::config::ApiConfig>().await {
                Ok(data) => {
                    _ = data;
                    tracing::info!("Client connected to Server!");
                    // At a later point, we might want to check certain elements
                    return Ok(true);
                }
                Err(_) => return Err(false),
            },
            _ => return Err(false),
        },
        Err(_) => return Err(false),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MicrosoftAuthCodeAndUrlResponse {
    pub url: String,
    pub state: String,
}

#[tauri::command(async)]
pub(crate) async fn microsoft_auth() -> Result<MicrosoftAuthCodeAndUrlResponse, bool> {
    match common::auth::xbl::client_authenticate_step_1(String::from(
        "a17f9693-f01f-4d1d-ad12-1f179478375d",
    ))
    .await
    {
        Ok((url, state)) => Ok(MicrosoftAuthCodeAndUrlResponse { url, state }),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn microsoft_auth_listener(state: String) -> Result<String, bool> {
    match common::auth::xbl::client_authenticate_step_2(state).await {
        Ok(code) => Ok(code),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(false)
        }
    }
}
