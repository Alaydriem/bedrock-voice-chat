use log::error;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Response, StatusCode,
};

use common::response::AudioFileResponse;
use common::response::auth::AuthStateResponse;
use common::response::error::ApiError;

use super::Api;

/// Extracts error messages from JSON API error responses.
struct ApiErrorResponse;

impl ApiErrorResponse {
    /// Extract error message from a structured API error response body,
    /// falling back to the status code if the body can't be parsed.
    async fn from_response(response: Response) -> String {
        let status = response.status();
        match response.text().await {
            Ok(body) => {
                if let Ok(api_error) = serde_json::from_str::<ApiError>(&body) {
                    return api_error.to_string();
                }
                format!("Server returned status: {}", status)
            }
            Err(_) => format!("Server returned status: {}", status),
        }
    }
}

impl Api {
    /// List all non-deleted audio files from the server.
    pub(crate) async fn list_audio_files(
        &self,
        game: Option<&str>,
    ) -> Result<Vec<AudioFileResponse>, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
        let url = format!("{}/api/audio/file", self.endpoint);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match client.get(url).headers(headers).send().await {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse audio file list: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(e) => {
                error!("Failed to list audio files: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Upload an Ogg/Opus audio file to the server.
    pub(crate) async fn upload_audio_file(
        &self,
        opus_bytes: Vec<u8>,
        filename: &str,
        game: Option<&str>,
    ) -> Result<AudioFileResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
        let url = format!("{}/api/audio/file", self.endpoint);

        // Send raw Opus bytes as POST body (server expects raw OggS data)
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/octet-stream"),
        );
        if let Ok(val) = HeaderValue::from_str(filename) {
            headers.insert("X-Original-Filename", val);
        }
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match client
            .post(url)
            .headers(headers)
            .body(opus_bytes)
            .send()
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse upload response: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(e) => {
                error!("Failed to upload audio file: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Delete an audio file from the server.
    pub(crate) async fn delete_audio_file(
        &self,
        file_id: &str,
        game: Option<&str>,
    ) -> Result<bool, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
        let url = format!("{}/api/audio/file/{}", self.endpoint, file_id);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match client.delete(url).headers(headers).send().await {
            Ok(response) if response.status() == StatusCode::OK => Ok(true),
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(e) => {
                error!("Failed to delete audio file: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Get the current server state (permissions) for the authenticated player.
    /// `game` is sent as X-Game header to help the server disambiguate legacy certs.
    pub(crate) async fn get_server_state(
        &self,
        game: Option<&str>,
    ) -> Result<AuthStateResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
        let url = format!("{}/api/auth/state", self.endpoint);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let body = response
                        .text()
                        .await
                        .map_err(|e| format!("Failed to read response: {}", e))?;
                    serde_json::from_str(&body)
                        .map_err(|e| format!("Failed to parse server state: {}", e))
                }
                _ => Err(ApiErrorResponse::from_response(response).await),
            },
            Err(e) => {
                error!("Failed to get server state: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
