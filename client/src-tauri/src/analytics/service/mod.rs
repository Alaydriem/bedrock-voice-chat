use std::sync::Arc;
use chrono::Utc;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use crate::analytics::AnalyticsProviderType;
use crate::analytics::dtos::QueuedEvent;
use crate::logging::Telemetry;

pub struct AnalyticsService {
    providers: Vec<AnalyticsProviderType>,
    queue: parking_lot::Mutex<Vec<QueuedEvent>>,
    telemetry: Arc<Telemetry>,
    install_id: String,
    session_id: String,
}

impl AnalyticsService {
    pub fn new(telemetry: Arc<Telemetry>, install_id: String) -> Self {
        Self {
            providers: Vec::new(),
            queue: parking_lot::Mutex::new(Vec::new()),
            telemetry,
            install_id,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn add_provider(&mut self, provider: AnalyticsProviderType) {
        self.providers.push(provider);
    }

    pub fn track(&self, event: AnalyticsEvent, data: Option<AnalyticsEventData>) {
        if !self.telemetry.is_enabled() {
            return;
        }

        let queued = QueuedEvent {
            event,
            properties: data,
            timestamp: Utc::now(),
        };

        self.queue.lock().push(queued);
    }

    pub async fn flush(&self) -> Result<(), anyhow::Error> {
        let events: Vec<QueuedEvent> = {
            let mut queue = self.queue.lock();
            queue.drain(..).collect()
        };

        if events.is_empty() {
            return Ok(());
        }

        let mut any_success = false;
        for provider in &self.providers {
            match provider.send_batch(&events, &self.install_id, &self.session_id).await {
                Ok(()) => any_success = true,
                Err(e) => log::warn!("Analytics provider flush failed: {}", e),
            }
        }

        if !any_success && !self.providers.is_empty() {
            let requeue_count = events.len();
            let mut queue = self.queue.lock();
            for event in events {
                queue.push(event);
            }
            log::warn!("All analytics providers failed. {} events re-queued.", requeue_count);
        }

        Ok(())
    }
}
