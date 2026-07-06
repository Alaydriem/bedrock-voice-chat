use std::time::Duration;

use common::structs::analytics::posthog::{BatchRequest, CaptureEvent};
use tokio::sync::mpsc::Receiver;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::services::metrics_service::event::TelemetryEvent;
use crate::services::metrics_service::posthog::properties::EventProperties;

const FLUSH_INTERVAL_SECS: u64 = 10;
const BATCH_MAX_EVENTS: usize = 100;
const CHUNK_SIZE: usize = 25;

pub struct PosthogClient {
    client: reqwest::Client,
    host: String,
    api_key: String,
    distinct_id: String,
    version: String,
}

impl PosthogClient {
    pub fn new(host: String, api_key: String, distinct_id: String, version: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            host,
            api_key,
            distinct_id,
            version,
        }
    }

    fn properties_for(&self, event: &TelemetryEvent) -> EventProperties {
        let mut props = EventProperties {
            server_version: self.version.clone(),
            hostname_sha: None,
            session_duration_secs: None,
        };
        match event {
            TelemetryEvent::ServerStarted { hostname_sha, .. } => {
                props.hostname_sha = Some(hostname_sha.clone());
            }
            TelemetryEvent::Disconnected { duration_secs, .. } => {
                props.session_duration_secs = Some(*duration_secs);
            }
            TelemetryEvent::Connected { .. }
            | TelemetryEvent::ChannelJoined { .. }
            | TelemetryEvent::ChannelLeft { .. } => {}
        }
        props
    }

    pub fn build_batch(&self, events: &[TelemetryEvent]) -> BatchRequest<EventProperties> {
        let batch = events
            .iter()
            .map(|e| CaptureEvent {
                event: e.name().to_string(),
                distinct_id: self.distinct_id.clone(),
                timestamp: e.at(),
                properties: self.properties_for(e),
            })
            .collect();

        BatchRequest {
            api_key: self.api_key.clone(),
            batch,
        }
    }

    /// Drains events until the channel closes or `shutdown` is cancelled, batching
    /// on an interval. A final flush runs on either exit so shutdown loses nothing.
    pub async fn run(self, mut rx: Receiver<TelemetryEvent>, shutdown: CancellationToken) {
        let mut buffer: Vec<TelemetryEvent> = Vec::new();
        let mut tick = interval(Duration::from_secs(FLUSH_INTERVAL_SECS));

        loop {
            tokio::select! {
                maybe = rx.recv() => {
                    match maybe {
                        Some(event) => {
                            buffer.push(event);
                            if buffer.len() >= BATCH_MAX_EVENTS {
                                self.flush(&mut buffer).await;
                            }
                        }
                        None => break,
                    }
                }
                _ = tick.tick() => self.flush(&mut buffer).await,
                _ = shutdown.cancelled() => break,
            }
        }

        self.flush(&mut buffer).await;
    }

    async fn flush(&self, buffer: &mut Vec<TelemetryEvent>) {
        if buffer.is_empty() {
            return;
        }
        let url = format!("{}/batch/", self.host);
        for chunk in buffer.chunks(CHUNK_SIZE) {
            let body = self.build_batch(chunk);
            match self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_client_error() || status.is_server_error() {
                        let text = resp.text().await.unwrap_or_default();
                        tracing::warn!("PostHog error: {} - {}", status, text);
                    }
                }
                Err(e) => tracing::warn!("PostHog request failed: {}", e),
            }
        }
        buffer.clear();
    }
}
