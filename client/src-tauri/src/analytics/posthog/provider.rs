use std::collections::HashMap;

use crate::analytics::dtos::QueuedEvent;
use crate::analytics::posthog::{BatchRequest, CaptureEvent, CaptureEventProperties};

pub struct Provider {
    client: reqwest::Client,
    host: String,
    api_key: String,
    app_version: String,
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
            is_debug: self.is_debug,
            custom,
        }
    }

    pub async fn send_batch(
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
            let batch: Vec<CaptureEvent> = chunk
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
