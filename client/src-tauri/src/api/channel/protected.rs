use crate::api::Api;

use crate::api::circuit_breaker::SendError;
use common::request::channel::ProtectedChannelCreateRequest;
use common::reqwest::StatusCode;
use log::error;

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
                error!("Protected channel creation failed with status: {}", response.status());
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
                    error!("Protected channel join failed with status: {}", status);
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
                error!("Protected channel request failed: {}", e);
                "Network error occurred".to_string()
            }
        }
    }
}
