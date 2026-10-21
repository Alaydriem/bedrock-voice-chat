use crate::api::Api;

use crate::api::circuit_breaker::SendError;
use common::request::channel::ProtectedChannelCreateRequest;
use common::reqwest::StatusCode;
use tauri_plugin_curia::curia;

impl Api {
    /// Creates an end-to-end encrypted channel. The request carries no password.
    pub(crate) async fn create_protected_channel(&self, name: String) -> Result<String, String> {
        let url = format!("{}/api/channel/protected", self.endpoint());
        let request = self
            .get_client()
            .post(url)
            .json(&ProtectedChannelCreateRequest { name });
        match self.send(request).await {
            Ok(response) if response.status() == StatusCode::OK => response
                .json::<String>()
                .await
                .map_err(|_| "Failed to parse response".to_string()),
            Ok(response) => {
                curia::error!("protected channel creation failed", {
                    status: response.status().as_u16(),
                });
                Err(format!("Request failed with status: {}", response.status()))
            }
            Err(e) => Err(Self::describe_send_error(e)),
        }
    }

    /// Joins an end-to-end encrypted channel's routing. Admission to its keys happens between
    /// members, over the key socket.
    pub(crate) async fn join_protected_channel(&self, channel_id: String) -> Result<bool, String> {
        let url = format!("{}/api/channel/{}/protected", self.endpoint(), channel_id);
        match self.send(self.get_client().put(url)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::NOT_FOUND => Err("Group not found".to_string()),
                StatusCode::CONFLICT => Err("This group is not protected".to_string()),
                status => {
                    curia::error!("protected channel join failed", {
                        channel_id: channel_id.clone(),
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => Err(Self::describe_send_error(e)),
        }
    }

    fn describe_send_error(error: SendError) -> String {
        match error {
            SendError::Open => "Server temporarily unreachable; backing off".to_string(),
            SendError::Transport(e) => {
                curia::error!("protected channel request failed", {
                    error: e.to_string(),
                });
                "Network error occurred".to_string()
            }
        }
    }
}
