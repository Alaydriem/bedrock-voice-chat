use common::structs::chat::ChatWorld;
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderValue};

use crate::api::Api;
use crate::api::circuit_breaker::SendError;

impl Api {
    /// Worlds this player has been seen in, newest first.
    ///
    /// Backs the app's chat target picker. Only reachable in net mode — the no-net path has no
    /// world key at all, because the proxy session is the world.
    pub(crate) async fn chat_worlds(&self) -> Result<Vec<ChatWorld>, String> {
        let client = self.get_client();
        let url = format!("{}/api/chat/worlds", self.endpoint);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) if response.status() == StatusCode::OK => response
                .json::<Vec<ChatWorld>>()
                .await
                .map_err(|e| format!("Could not decode the world list: {e}")),
            Ok(response) => Err(format!("Server returned status: {}", response.status())),
            // The breaker tripping and the network failing mean different things to somebody
            // deciding whether to retry, so they do not collapse into one message.
            Err(SendError::Open) => Err("The server is not responding".to_string()),
            Err(SendError::Transport(e)) => Err(format!("Could not reach the server: {e}")),
        }
    }
}
