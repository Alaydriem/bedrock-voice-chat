use log::error;
use reqwest::{
    Response, StatusCode,
    header::{HeaderMap, HeaderValue},
};

use common::request::AudioFileListQuery;
use common::response::{ApiError, AudioFileResponse, AudioStreamTokenResponse, PaginatedResponse};

use super::Api;

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
        let client = self.get_client(Some(self.endpoint.as_str())).await;
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

        match client.get(url).query(&query_params).headers(headers).send().await {
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

    pub(crate) async fn upload_audio_file(
        &self,
        opus_bytes: Vec<u8>,
        filename: &str,
        game: Option<&str>,
    ) -> Result<AudioFileResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
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

        match client
            .post(url)
            .headers(headers)
            .body(opus_bytes)
            .send()
            .await
        {
            Ok(response) if response.status() == StatusCode::CREATED || response.status() == StatusCode::OK => {
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

    pub(crate) async fn get_server_state(
        &self,
        game: Option<&str>,
    ) -> Result<common::response::auth::AuthStateResponse, String> {
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
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse server state: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(e) => {
                error!("Failed to get server state: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    pub(crate) async fn get_audio_stream_token(
        &self,
        file_id: &str,
        game: Option<&str>,
    ) -> Result<AudioStreamTokenResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;
        let url = format!("{}/api/audio/file/{}/token", self.endpoint, file_id);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        if let Some(game) = game {
            if let Ok(val) = HeaderValue::from_str(game) {
                headers.insert("X-Game", val);
            }
        }

        match client.post(url).headers(headers).send().await {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body)
                    .map_err(|e| format!("Failed to parse stream token: {}", e))
            }
            Ok(response) => Err(ApiErrorResponse::from_response(response).await),
            Err(e) => {
                error!("Failed to get audio stream token: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
