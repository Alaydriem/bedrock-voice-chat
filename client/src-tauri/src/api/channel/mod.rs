use crate::api::Api;

use log::error;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    StatusCode,
};
use serde_json::json;
use std::error::Error;

impl Api {
    /// Creates a new channel
    pub(crate) async fn create_channel(&self, name: String) -> Result<String, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel", self.endpoint);
        let body = json!(name);

        match client.post(url).headers(headers).json(&body).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response.json::<String>().await {
                        Ok(channel_id) => Ok(channel_id),
                        Err(e) => {
                            error!("Failed to parse channel creation response: {}", e);
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                status => {
                    error!("Channel creation failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to create channel: {}", e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Deletes a channel (owner only)
    pub(crate) async fn delete_channel(&self, channel_id: String) -> Result<bool, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel/{}", self.endpoint, channel_id);

        match client.delete(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::UNAUTHORIZED => {
                    error!("Not authorized to delete channel {}", channel_id);
                    Err("You are not authorized to delete this channel".to_string())
                }
                StatusCode::NOT_FOUND => {
                    error!("Channel {} not found", channel_id);
                    Err("Channel not found".to_string())
                }
                status => {
                    error!("Channel deletion failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to delete channel: {}", e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Lists all channels
    pub(crate) async fn list_channels(&self) -> Result<Vec<common::structs::channel::Channel>, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel", self.endpoint);

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response.json::<Vec<common::structs::channel::Channel>>().await {
                        Ok(channels) => Ok(channels),
                        Err(e) => {
                            error!("Failed to parse channels list response: {}", e);
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                status => {
                    error!("Channel list failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to list channels: {}", e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Gets a specific channel by ID
    pub(crate) async fn get_channel(&self, channel_id: &str) -> Result<common::structs::channel::Channel, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel?id={}", self.endpoint, channel_id);

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    match response.json::<Vec<common::structs::channel::Channel>>().await {
                        Ok(mut channels) => {
                            if let Some(channel) = channels.pop() {
                                Ok(channel)
                            } else {
                                Err("Channel not found".to_string())
                            }
                        },
                        Err(e) => {
                            error!("Failed to parse channel response: {}", e);
                            Err("Failed to parse response".to_string())
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    Err("Channel not found".to_string())
                }
                status => {
                    error!("Get channel failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to get channel: {}", e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
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
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/channel/{}", self.endpoint, channel_id);

        match client.put(url).headers(headers).json(&event).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(true),
                StatusCode::BAD_REQUEST => {
                    error!("Bad request for channel event on channel {}", channel_id);
                    Err("Invalid channel or event".to_string())
                }
                StatusCode::FORBIDDEN => {
                    error!("Not authorized for channel event on channel {}", channel_id);
                    Err("You are not authorized to perform this action".to_string())
                }
                status => {
                    error!("Channel event failed with status: {}", status);
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(e) => {
                error!("Failed to send channel event: {}", e);
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