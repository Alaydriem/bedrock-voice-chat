use reqwest::{
    Response, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use tauri_plugin_curia::curia;

use common::request::AudioFileListQuery;
use common::response::{ApiError, AudioFileResponse, AudioStreamTokenResponse, PaginatedResponse};

use super::Api;
use super::circuit_breaker::SendError;

struct ApiErrorResponse;

impl ApiErrorResponse {
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
    pub(crate) async fn list_audio_files(
        &self,
        game: Option<&str>,
        query: &AudioFileListQuery,
    ) -> Result<PaginatedResponse<AudioFileResponse>, String> {
        let client = self.get_client();
        let url = format!("{}/api/audio/file", self.endpoint);

        let mut query_params: Vec<(&str, String)> = Vec::new();
        if let Some(page) = query.page {
            query_params.push(("page", page.to_string()));
        }
        if let Some(page_size) = query.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }
        if let Some(ref sort_by) = query.sort_by {
            query_params.push(("sort_by", sort_by.clone()));
        }
        if let Some(ref sort_order) = query.sort_order {
            query_params.push(("sort_order", sort_order.clone()));
        }
        if let Some(ref search) = query.search {
            query_params.push(("search", search.clone()));
        }

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match self
            .send(client.get(url).query(&query_params).headers(headers))
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse audio file list: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                curia::error!("failed to list audio files", {
                    error: e.to_string(),
                });
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    pub(crate) async fn upload_audio_file(
        &self,
        opus_bytes: Vec<u8>,
        filename: &str,
        game: Option<&str>,
    ) -> Result<AudioFileResponse, String> {
        let client = self.get_client();
        let url = format!("{}/api/audio/file", self.endpoint);

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

        match self
            .send(client.post(url).headers(headers).body(opus_bytes))
            .await
        {
            Ok(response)
                if response.status() == StatusCode::CREATED
                    || response.status() == StatusCode::OK =>
            {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse upload response: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                curia::error!("failed to upload an audio file", {
                    error: e.to_string(),
                });
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    pub(crate) async fn delete_audio_file(
        &self,
        file_id: &str,
        game: Option<&str>,
    ) -> Result<bool, String> {
        let client = self.get_client();
        let url = format!("{}/api/audio/file/{}", self.endpoint, file_id);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match self.send(client.delete(url).headers(headers)).await {
            Ok(response) if response.status() == StatusCode::OK => Ok(true),
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                curia::error!("failed to delete an audio file", {
                    error: e.to_string(),
                });
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Ask the server whether it still accepts this device's certificate.
    ///
    /// Used before signing a player out over a rejected voice handshake. Deliberately separate
    /// from `get_server_state`, which flattens every outcome to `Result<_, String>` and so
    /// collapses a refused certificate and an unreachable server into the same value — the one
    /// distinction this decision rests on.
    pub(crate) async fn verify_credentials(&self) -> crate::api::CredentialVerdict {
        use crate::api::CredentialVerdict;

        let client = self.get_client();
        let url = format!("{}/api/auth/state", self.endpoint);

        match self.send(client.get(url)).await {
            Ok(response) if response.status().is_success() => CredentialVerdict::Valid,
            Ok(response)
                if response.status() == StatusCode::UNAUTHORIZED
                    || response.status() == StatusCode::FORBIDDEN =>
            {
                CredentialVerdict::Rejected
            }
            Ok(response) => {
                curia::warn!("credential probe was inconclusive", {
                    connected_server: self.endpoint.clone(),
                    status: response.status().as_u16(),
                });
                CredentialVerdict::Inconclusive
            }
            Err(SendError::Transport(e)) if crate::network::CredentialFault::in_tls_chain(&e) => {
                CredentialVerdict::Rejected
            }
            Err(SendError::Transport(e)) => {
                curia::warn!("credential probe was inconclusive", {
                    connected_server: self.endpoint.clone(),
                    error: e.to_string(),
                });
                CredentialVerdict::Inconclusive
            }
            Err(SendError::Open) => {
                curia::warn!("credential probe was inconclusive: the endpoint breaker is open", {
                    connected_server: self.endpoint.clone(),
                });
                CredentialVerdict::Inconclusive
            }
        }
    }

    pub(crate) async fn get_server_state(
        &self,
        game: Option<&str>,
    ) -> Result<common::response::auth::AuthStateResponse, String> {
        let client = self.get_client();
        let url = format!("{}/api/auth/state", self.endpoint);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse server state: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                curia::error!("failed to get the server state", {
                    error: e.to_string(),
                });
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    pub(crate) async fn get_audio_stream_token(
        &self,
        file_id: &str,
        game: Option<&str>,
    ) -> Result<AudioStreamTokenResponse, String> {
        let client = self.get_client();
        let url = format!("{}/api/audio/file/{}/token", self.endpoint, file_id);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match self.send(client.post(url).headers(headers)).await {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse stream token: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                curia::error!("failed to get an audio stream token", {
                    error: e.to_string(),
                });
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
