use crate::analytics::AnalyticsProviderType;
use crate::analytics::PlayerIdentity;
use crate::analytics::dtos::QueuedEvent;
use crate::logging::Telemetry;
use chrono::Utc;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use std::sync::Arc;

#[derive(Default, Clone)]
struct AnalyticsContext {
    connected_server: Option<String>,
    player: Option<PlayerIdentity>,
}

pub struct AnalyticsService {
    providers: Vec<AnalyticsProviderType>,
    queue: parking_lot::Mutex<Vec<QueuedEvent>>,
    context: parking_lot::RwLock<AnalyticsContext>,
    telemetry: Arc<Telemetry>,
    install_id: String,
    session_id: String,
}

impl AnalyticsService {
    pub fn new(telemetry: Arc<Telemetry>, install_id: String) -> Self {
        Self {
            providers: Vec::new(),
            queue: parking_lot::Mutex::new(Vec::new()),
            context: parking_lot::RwLock::new(AnalyticsContext::default()),
            telemetry,
            install_id,
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
        sentry::configure_scope(|scope| match &server {
            Some(s) => scope.set_tag("connected_server", s),
            None => scope.remove_tag("connected_server"),
        });
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
        sentry::configure_scope(|scope| {
            scope.set_tag("player_display", &display);
            scope.set_tag("player_hash", &hash);
        });
    }

    pub fn clear_player(&self) {
        {
            let mut ctx = self.context.write();
            ctx.player = None;
        }
        sentry::configure_scope(|scope| {
            scope.remove_tag("player_display");
            scope.remove_tag("player_hash");
        });
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

        let mut any_success = false;
        for provider in &self.providers {
            match provider
                .send_batch(&events, &self.install_id, &self.session_id)
                .await
            {
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
            log::warn!(
                "All analytics providers failed. {} events re-queued.",
                requeue_count
            );
        }

        Ok(())
    }
}
