use std::collections::HashMap;

use async_trait::async_trait;

use crate::analytics::AnalyticsLevel;
use crate::analytics::dtos::QueuedEvent;
use crate::analytics::posthog::{BatchRequest, CaptureEvent, CaptureEventProperties};
use crate::analytics::provider::AnalyticsProvider;

pub struct Provider {
    client: reqwest::Client,
    host: String,
    api_key: String,
    app_version: String,
    app_build: String,
    os: String,
    is_debug: bool,
}

impl Provider {
    pub fn new(host: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            host,
            api_key,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            app_build: option_env!("APP_BUILD_NUMBER").unwrap_or("local").to_string(),
            os: std::env::consts::OS.to_string(),
            is_debug: cfg!(debug_assertions),
        }
    }

    fn build_properties(&self, event: &QueuedEvent, session_id: &str) -> CaptureEventProperties {
        let custom = match &event.properties {
            Some(data) => data.properties.clone(),
            None => HashMap::new(),
        };

        CaptureEventProperties {
            session_id: session_id.to_string(),
            os: self.os.clone(),
            app_version: self.app_version.clone(),
            app_build: self.app_build.clone(),
            is_debug: self.is_debug,
            connected_server: event.connected_server.clone(),
            player_display: event.player_display.clone(),
            player_hash: event.player_hash.clone(),
            custom,
        }
    }
}

#[async_trait]
impl AnalyticsProvider for Provider {
    fn set_tag(&self, _key: &str, _value: &str) {}

    fn clear_tag(&self, _key: &str) {}

    fn set_user(&self, _user_id: &str) {}

    fn breadcrumb(&self, _category: &str, _message: &str, _level: AnalyticsLevel) {}

    fn capture_message(&self, _message: &str, _level: AnalyticsLevel, _tags: &[(String, String)]) {}

    async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        if events.is_empty() {
            return Ok(());
        }

        let url = format!("{}/batch/", self.host);

        for chunk in events.chunks(25) {
            let batch: Vec<CaptureEvent<CaptureEventProperties>> = chunk
                .iter()
                .map(|e| CaptureEvent {
                    event: e.event.name().to_string(),
                    distinct_id: install_id.to_string(),
                    timestamp: e.timestamp,
                    properties: self.build_properties(e, session_id),
                })
                .collect();

            let body = BatchRequest {
                api_key: self.api_key.clone(),
                batch,
            };

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_server_error() || status.is_client_error() {
                        let body = resp.text().await.unwrap_or_default();
                        log::warn!("PostHog error: {} - {}", status, body);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "PostHog request failed: {}. Continuing with remaining chunks.",
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
