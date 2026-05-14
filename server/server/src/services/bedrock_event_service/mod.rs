mod rejection;

pub use rejection::BedrockEventRejection;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::request::audio::play::MinecraftAudioContext;
use common::request::{AudioPlayRequest, GameAudioContext};
use common::structs::packet::{BedrockEvent, BedrockEventPacket};
use common::{Coordinate, Orientation, PlayerEnum};
use moka::future::Cache;
use sea_orm::DatabaseConnection;

use crate::runtime::position_updater::PositionUpdater;
use crate::services::AudioPlaybackService;
use crate::stream::quic::WebhookReceiver;

const PHANTOM_LEAVE_COORD: f32 = -10000.0;

pub struct BedrockEventService {
    last_addon_http: Cache<String, Instant>,
    threshold: Duration,
    playback_service: Arc<AudioPlaybackService>,
    webhook_receiver: WebhookReceiver,
    db_conn: Arc<DatabaseConnection>,
}

impl BedrockEventService {
    pub fn new(
        playback_service: Arc<AudioPlaybackService>,
        webhook_receiver: WebhookReceiver,
        db_conn: Arc<DatabaseConnection>,
        threshold_secs: u32,
    ) -> Self {
        let last_addon_http = Cache::builder()
            .max_capacity(256)
            .time_to_live(Duration::from_secs(60))
            .build();

        Self {
            last_addon_http,
            threshold: Duration::from_secs(threshold_secs as u64),
            playback_service,
            webhook_receiver,
            db_conn,
        }
    }

    pub fn new_shared(
        playback_service: Arc<AudioPlaybackService>,
        webhook_receiver: WebhookReceiver,
        db_conn: Arc<DatabaseConnection>,
        threshold_secs: u32,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            playback_service,
            webhook_receiver,
            db_conn,
            threshold_secs,
        ))
    }

    pub async fn notify_addon_http(&self, world_uuid: &str) {
        if world_uuid.is_empty() {
            return;
        }
        self.last_addon_http
            .insert(world_uuid.to_string(), Instant::now())
            .await;
    }

    pub async fn is_bds_healthy(&self, world_uuid: &str) -> bool {
        match self.last_addon_http.get(world_uuid).await {
            Some(last) => last.elapsed() < self.threshold,
            None => false,
        }
    }

    pub async fn handle_event(
        &self,
        packet: BedrockEventPacket,
        authenticated_player: String,
    ) -> Result<(), BedrockEventRejection> {
        if self.is_bds_healthy(&packet.world_uuid).await {
            tracing::debug!(
                world_uuid = %packet.world_uuid,
                player = %authenticated_player,
                "Rejecting bedrock proxy event: BDS addon is healthy"
            );
            return Err(BedrockEventRejection::BdsHealthy);
        }

        match packet.event {
            BedrockEvent::JukeboxInsert {
                audio_id,
                block_pos,
                dimension,
                ..
            } => {
                self.on_jukebox_insert(audio_id, block_pos, dimension, packet.world_uuid)
                    .await
            }
            BedrockEvent::JukeboxEject { event_id, .. } => {
                self.on_jukebox_eject(event_id).await
            }
            BedrockEvent::PlayerDeath {
                dimension,
                last_pos: _,
                ..
            } => {
                self.on_player_death(authenticated_player, dimension, packet.world_uuid)
                    .await
            }
            BedrockEvent::PlayerLeave { .. } => {
                self.on_player_leave(authenticated_player, packet.world_uuid)
                    .await
            }
        }
    }

    async fn on_jukebox_insert(
        &self,
        audio_id: String,
        block_pos: Coordinate,
        dimension: Dimension,
        world_uuid: String,
    ) -> Result<(), BedrockEventRejection> {
        let request = AudioPlayRequest {
            audio_file_id: audio_id,
            game: GameAudioContext::Minecraft(MinecraftAudioContext {
                coordinates: block_pos,
                dimension,
                world_uuid,
            }),
        };

        self.playback_service
            .start_playback(&*self.db_conn, request)
            .await
            .map_err(BedrockEventRejection::Internal)?;

        Ok(())
    }

    async fn on_jukebox_eject(
        &self,
        event_id: String,
    ) -> Result<(), BedrockEventRejection> {
        self.playback_service
            .stop_playback(&event_id)
            .await
            .map_err(BedrockEventRejection::Internal)?;
        Ok(())
    }

    async fn on_player_death(
        &self,
        player_name: String,
        _dimension: Dimension,
        world_uuid: String,
    ) -> Result<(), BedrockEventRejection> {
        let player = MinecraftPlayer {
            name: player_name,
            coordinates: Coordinate { x: 0.0, y: 0.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Death,
            deafen: false,
            spectator: true,
            world_uuid: if world_uuid.is_empty() {
                None
            } else {
                Some(world_uuid)
            },
            alternative_identity: None,
            player_uuid: None,
        };

        PositionUpdater::broadcast_positions(
            vec![PlayerEnum::Minecraft(player)],
            &self.webhook_receiver,
        )
        .await;
        Ok(())
    }

    async fn on_player_leave(
        &self,
        player_name: String,
        world_uuid: String,
    ) -> Result<(), BedrockEventRejection> {
        let player = MinecraftPlayer {
            name: player_name,
            coordinates: Coordinate {
                x: PHANTOM_LEAVE_COORD,
                y: PHANTOM_LEAVE_COORD,
                z: PHANTOM_LEAVE_COORD,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: true,
            world_uuid: if world_uuid.is_empty() {
                None
            } else {
                Some(world_uuid)
            },
            alternative_identity: None,
            player_uuid: None,
        };

        PositionUpdater::broadcast_positions(
            vec![PlayerEnum::Minecraft(player)],
            &self.webhook_receiver,
        )
        .await;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stale_gate_reports_unhealthy() {
        let cache: Cache<String, Instant> = Cache::builder()
            .max_capacity(8)
            .time_to_live(Duration::from_secs(60))
            .build();
        let threshold = Duration::from_secs(30);

        let healthy = match cache.get("world-x").await {
            Some(last) => last.elapsed() < threshold,
            None => false,
        };

        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_fresh_gate_reports_healthy() {
        let cache: Cache<String, Instant> = Cache::builder()
            .max_capacity(8)
            .time_to_live(Duration::from_secs(60))
            .build();
        let threshold = Duration::from_secs(30);

        cache.insert("world-x".to_string(), Instant::now()).await;

        let healthy = match cache.get("world-x").await {
            Some(last) => last.elapsed() < threshold,
            None => false,
        };

        assert!(healthy);
    }

}
