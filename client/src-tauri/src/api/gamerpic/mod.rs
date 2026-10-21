use crate::api::Api;

use crate::api::circuit_breaker::SendError;
use common::reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};
use common::response::GamerpicResponse;
use std::error::Error;
use tauri_plugin_curia::curia;

impl Api {
    pub(crate) async fn get_gamerpic(
        &self,
        game: &str,
        gamertag: &str,
    ) -> Result<GamerpicResponse, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/gamerpic/{}/{}", self.endpoint, game, gamertag);

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => match response.json::<GamerpicResponse>().await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        curia::error!("failed to parse the gamerpic response", {
                            error: e.to_string(),
                        });
                        Err("Failed to parse response".to_string())
                    }
                },
                status => {
                    curia::error!("gamerpic request failed", {
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                let mut causes = Vec::new();
                let mut source = e.source();
                while let Some(cause) = source {
                    causes.push(cause.to_string());
                    source = cause.source();
                }
                curia::error!("failed to get gamerpic", {
                    error: e.to_string(),
                    causes: causes,
                });
                Err("Network error occurred".to_string())
            }
        }
    }
}
