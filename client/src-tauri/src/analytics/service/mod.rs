use crate::analytics::AnalyticsLevel;
use crate::analytics::AnalyticsProvider;
use crate::analytics::AnalyticsProviderType;
use crate::analytics::PlatformId;
use crate::analytics::PlayerIdentity;
use crate::analytics::dtos::QueuedEvent;
use crate::logging::Telemetry;
use chrono::Utc;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use futures_util::future::join_all;
use std::sync::Arc;
mod context;

use context::AnalyticsContext;


pub struct AnalyticsService {
    providers: Vec<AnalyticsProviderType>,
    queue: parking_lot::Mutex<Vec<QueuedEvent>>,
    context: parking_lot::RwLock<AnalyticsContext>,
    telemetry: Arc<Telemetry>,
    platform_id: Arc<PlatformId>,
    session_id: String,
}

impl AnalyticsService {
    pub fn new(telemetry: Arc<Telemetry>, platform_id: Arc<PlatformId>) -> Self {
        Self {
            providers: Vec::new(),
            queue: parking_lot::Mutex::new(Vec::new()),
            context: parking_lot::RwLock::new(AnalyticsContext::default()),
            telemetry,
            platform_id,
            session_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn add_provider(&mut self, provider: AnalyticsProviderType) {
        self.providers.push(provider);
    }

    pub fn set_connected_server(&self, server: Option<String>) {
        {
            let mut ctx = self.context.write();
            ctx.connected_server = server.clone();
        }
        for provider in &self.providers {
            match &server {
                Some(s) => provider.set_tag("connected_server", s),
                None => provider.clear_tag("connected_server"),
            }
        }
    }

    pub fn clear_connected_server(&self) {
        self.set_connected_server(None);
    }

    pub fn set_player(&self, gamertag: &str) {
        let identity = PlayerIdentity::from_gamertag(gamertag);
        let display = identity.display.clone();
        let hash = identity.hash.clone();
        {
            let mut ctx = self.context.write();
            ctx.player = Some(identity);
        }
        for provider in &self.providers {
            provider.set_tag("player_display", &display);
            provider.set_tag("player_hash", &hash);
        }
    }

    pub fn clear_player(&self) {
        {
            let mut ctx = self.context.write();
            ctx.player = None;
        }
        for provider in &self.providers {
            provider.clear_tag("player_display");
            provider.clear_tag("player_hash");
        }
    }

    pub fn set_user(&self, user_id: &str) {
        for provider in &self.providers {
            provider.set_user(user_id);
        }
    }

    pub fn breadcrumb(&self, category: &str, message: &str, level: AnalyticsLevel) {
        if !self.telemetry.is_enabled() {
            return;
        }
        for provider in &self.providers {
            provider.breadcrumb(category, message, level);
        }
    }

    pub fn capture_message(&self, message: &str, level: AnalyticsLevel, tags: &[(String, String)]) {
        if !self.telemetry.is_enabled() {
            return;
        }
        for provider in &self.providers {
            provider.capture_message(message, level, tags);
        }
    }

    pub fn track(&self, event: AnalyticsEvent, data: Option<AnalyticsEventData>) {
        if !self.telemetry.is_enabled() {
            return;
        }

        let (connected_server, player_display, player_hash) = {
            let ctx = self.context.read();
            (
                ctx.connected_server.clone(),
                ctx.player.as_ref().map(|p| p.display.clone()),
                ctx.player.as_ref().map(|p| p.hash.clone()),
            )
        };

        let queued = QueuedEvent {
            event,
            properties: data,
            timestamp: Utc::now(),
            connected_server,
            player_display,
            player_hash,
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

        let batch_providers: Vec<&AnalyticsProviderType> = self
            .providers
            .iter()
            .filter(|p| p.handles_batches())
            .collect();

        if batch_providers.is_empty() {
            return Ok(());
        }

        // Read at send time rather than held, so a batch queued before an identity
        // change is not attributed to the retired id.
        let platform_id = self.platform_id.get();
        let results = join_all(
            batch_providers
                .iter()
                .map(|p| p.send_batch(&events, &platform_id, &self.session_id)),
        )
        .await;

        let any_success = results.iter().any(|r| r.is_ok());
        for err in results.iter().filter_map(|r| r.as_ref().err()) {
            log::warn!("Analytics provider flush failed: {}", err);
        }

        if !any_success {
            let requeue_count = events.len();
            let mut queue = self.queue.lock();
            queue.extend(events);
            log::warn!(
                "All analytics providers failed. {} events re-queued.",
                requeue_count
            );
        }

        Ok(())
    }
}
