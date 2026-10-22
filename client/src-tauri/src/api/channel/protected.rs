use crate::api::Api;

use crate::api::circuit_breaker::SendError;
use common::reqwest::StatusCode;
use tauri_plugin_curia::curia;

impl Api {
    /// Turns this player's open channel into an end-to-end encrypted one. The error is the
    /// server's refusal code: `not_found`, `not_creator`, `not_alone` or `already_protected`.
    pub(crate) async fn protect_channel(&self, channel_id: String) -> Result<(), String> {
        let url = format!("{}/api/channel/{}/protect", self.endpoint(), channel_id);
        match self.send(self.get_client().put(url)).await {
            Ok(response) if response.status() == StatusCode::OK => Ok(()),
            Ok(response) => {
                let status = response.status();
                let code = response
                    .json::<String>()
                    .await
                    .unwrap_or_default();
                match code.as_str() {
                    "not_found" | "not_creator" | "not_alone" | "already_protected" => Err(code),
                    _ => {
                        curia::error!("protecting a channel failed", {
                            channel_id: channel_id.clone(),
                            status: status.as_u16(),
                        });
                        Err(format!("Request failed with status: {}", status))
                    }
                }
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
