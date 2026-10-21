mod protected;

use crate::api::Api;

use crate::api::circuit_breaker::SendError;
use common::reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};
use serde_json::json;
use std::error::Error;
use tauri_plugin_curia::curia;

impl Api {
    /// Creates a new channel
    pub(crate) async fn create_channel(&self, name: String) -> Result<String, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel", self.endpoint);
        let body = json!(name);

        match self
            .send(client.post(url).headers(headers).json(&body))
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => match response.json::<String>().await {
                    Ok(channel_id) => Ok(channel_id),
                    Err(e) => {
                        curia::error!("failed to parse the channel creation response", {
                            error: e.to_string(),
                        });
                        Err("Failed to parse response".to_string())
                    }
                },
                status => {
                    curia::error!("channel creation failed", {
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to create a channel", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Deletes a channel (owner only)
    pub(crate) async fn delete_channel(&self, channel_id: String) -> Result<bool, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel/{}", self.endpoint, channel_id);

        match self.send(client.delete(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::UNAUTHORIZED => {
                    curia::error!("not authorized to delete the channel", {
                        channel_id: channel_id.clone(),
                    });
                    Err("You are not authorized to delete this channel".to_string())
                }
                StatusCode::NOT_FOUND => {
                    curia::error!("channel not found", {
                        channel_id: channel_id.clone(),
                    });
                    Err("Channel not found".to_string())
                }
                status => {
                    curia::error!("channel deletion failed", {
                        channel_id: channel_id.clone(),
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to delete a channel", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Lists all channels
    pub(crate) async fn list_channels(
        &self,
    ) -> Result<Vec<common::structs::channel::Channel>, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel", self.endpoint);

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response
                        .json::<Vec<common::structs::channel::Channel>>()
                        .await
                    {
                        Ok(channels) => Ok(channels),
                        Err(e) => {
                            curia::error!("failed to parse the channel list response", {
                                error: e.to_string(),
                            });
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                status => {
                    curia::error!("channel list failed", {
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to list channels", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Gets a specific channel by ID
    pub(crate) async fn get_channel(
        &self,
        channel_id: &str,
    ) -> Result<common::structs::channel::Channel, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel?id={}", self.endpoint, channel_id);

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response
                        .json::<Vec<common::structs::channel::Channel>>()
                        .await
                    {
                        Ok(mut channels) => {
                            if let Some(channel) = channels.pop() {
                                Ok(channel)
                            } else {
                                Err("Channel not found".to_string())
                            }
                        }
                        Err(e) => {
                            curia::error!("failed to parse the channel response", {
                                channel_id: channel_id,
                                error: e.to_string(),
                            });
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                StatusCode::NOT_FOUND => Err("Channel not found".to_string()),
                status => {
                    curia::error!("get channel failed", {
                        channel_id: channel_id,
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to get a channel", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Renames a channel (owner only)
    pub(crate) async fn rename_channel(
        &self,
        channel_id: &str,
        name: &str,
    ) -> Result<bool, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel/{}", self.endpoint, channel_id);
        let body = json!(name);

        match self
            .send(client.patch(url).headers(headers).json(&body))
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::UNAUTHORIZED => {
                    curia::error!("not authorized to rename the channel", {
                        channel_id: channel_id,
                    });
                    Err("You are not authorized to rename this channel".to_string())
                }
                StatusCode::NOT_FOUND => {
                    curia::error!("channel not found", {
                        channel_id: channel_id,
                    });
                    Err("Channel not found".to_string())
                }
                status => {
                    curia::error!("channel rename failed", {
                        channel_id: channel_id,
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to rename a channel", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Sends a channel event (join/leave)
    pub(crate) async fn channel_event(
        &self,
        channel_id: String,
        event: common::structs::channel::ChannelEvent,
    ) -> Result<bool, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel/{}", self.endpoint, channel_id);

        match self
            .send(client.put(url).headers(headers).json(&event))
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::BAD_REQUEST => {
                    curia::error!("bad request for a channel event", {
                        channel_id: channel_id.clone(),
                    });
                    Err("Invalid channel or event".to_string())
                }
                StatusCode::FORBIDDEN => {
                    curia::error!("not authorized for a channel event", {
                        channel_id: channel_id.clone(),
                    });
                    Err("You are not authorized to perform this action".to_string())
                }
                status => {
                    curia::error!("channel event failed", {
                        channel_id: channel_id.clone(),
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                Self::channel_transport_failure("failed to send a channel event", &e);
                Err("Network error occurred".to_string())
            }
        }
    }

    // One event per failure, with the cause chain as a field, so the causes stay attached to
    // the failure they explain.
    fn channel_transport_failure(message: &'static str, error: &common::reqwest::Error) {
        let mut causes = Vec::new();
        let mut source = error.source();
        while let Some(cause) = source {
            causes.push(cause.to_string());
            source = cause.source();
        }
        curia::error!(message, {
            error: error.to_string(),
            causes: causes,
        });
    }
}
