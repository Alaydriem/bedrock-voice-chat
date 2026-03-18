use crate::api::Api;

use common::response::GamerpicResponse;
use log::error;
use common::reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use std::error::Error;

impl Api {
    pub(crate) async fn get_gamerpic(
        &self,
        game: &str,
        gamertag: &str,
    ) -> Result<GamerpicResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/gamerpic/{}/{}", self.endpoint, game, gamertag);

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response.json::<GamerpicResponse>().await {
                        Ok(result) => Ok(result),
                        Err(e) => {
                            error!("Failed to parse gamerpic response: {}", e);
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                status => {
                    error!("Gamerpic request failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to get gamerpic: {}", e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
                Err("Network error occurred".to_string())
            }
        }
    }
}
