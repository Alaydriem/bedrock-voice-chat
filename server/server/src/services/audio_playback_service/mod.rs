mod eject_scheduler;
mod playback_entry;
mod playback_expiry;
mod playback_task;
pub(crate) mod ogg_opus_parser;

pub use eject_scheduler::EjectScheduler;

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use common::players::{HytalePlayer, MinecraftPlayer};
use common::request::{AudioPlayRequest, GameAudioContext};
use common::response::AudioEventResponse;
use common::{Orientation, PlayerEnum};
use entity::audio_file;
use moka::future::Cache;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use tokio_util::sync::CancellationToken;

use crate::stream::quic::WebhookReceiver;

use ogg_opus_parser::OggOpusParser;
use playback_entry::PlaybackEntry;
use playback_expiry::PlaybackExpiry;
use playback_task::PlaybackTask;

pub struct AudioPlaybackService {
    active_playbacks: Cache<String, PlaybackEntry>,
    dedup_cache: Cache<String, String>,
    webhook_receiver: WebhookReceiver,
    audio_storage_path: String,
    parent_token: CancellationToken,
    eject_scheduler: OnceLock<Arc<EjectScheduler>>,
}

impl AudioPlaybackService {
    pub fn new(
        webhook_receiver: WebhookReceiver,
        audio_storage_path: String,
        parent_token: CancellationToken,
        _max_concurrent_per_uuid: usize,
    ) -> Self {
        Self {
            active_playbacks: Cache::builder()
                .max_capacity(10000)
                .expire_after(PlaybackExpiry)
                .build(),
            dedup_cache: Cache::builder()
                .max_capacity(10000)
                .time_to_live(Duration::from_secs(2))
                .build(),
            webhook_receiver,
            audio_storage_path,
            parent_token,
            eject_scheduler: OnceLock::new(),
        }
    }

    pub fn set_eject_scheduler(&self, scheduler: Arc<EjectScheduler>) {
        let _ = self.eject_scheduler.set(scheduler);
    }

    pub async fn start_playback<C: ConnectionTrait>(
        &self,
        conn: &C,
        request: AudioPlayRequest,
    ) -> Result<AudioEventResponse, String> {
        let dedup_key = match &request.game {
            GameAudioContext::Minecraft(ctx) => format!(
                "minecraft:{}:{}:{}:{}:{}",
                ctx.world_uuid, ctx.coordinates.x, ctx.coordinates.y, ctx.coordinates.z, request.audio_file_id
            ),
            GameAudioContext::Hytale(_) => format!("hytale:{}", request.audio_file_id),
        };

        if let Some(existing_event_id) = self.dedup_cache.get(&dedup_key).await {
            if self.active_playbacks.get(&existing_event_id).await.is_some() {
                return Err("Duplicate play request".to_string());
            }
        }

        let audio_file_id = request.audio_file_id.clone();
        let audio_file = audio_file::Entity::find_by_id(audio_file_id.clone())
            .filter(audio_file::Column::Deleted.eq(0))
            .one(conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| "Audio file not found".to_string())?;

        let file_path = format!("{}/{}.opus", self.audio_storage_path, audio_file.id);

        let (frames, duration_ms) =
            tokio::task::spawn_blocking(move || OggOpusParser::parse_frames(&file_path))
                .await
                .map_err(|e| format!("Task join error: {}", e))?
                .map_err(|e| format!("Ogg parsing error: {}", e))?;

        if frames.is_empty() {
            return Err("No audio frames found in file".to_string());
        }

        let event_id = uuid::Uuid::now_v7().to_string();

        tracing::info!(
            event_id = %event_id,
            file_id = %audio_file_id,
            duration_ms = duration_ms,
            frames = frames.len(),
            "Starting audio playback"
        );

        let stripped = event_id.replace('-', "");
        let jukebox_hash = &stripped[stripped.len() - 8..];
        let jukebox_name = format!("{}{}", common::consts::audio::JUKEBOX_PLAYER_PREFIX, jukebox_hash);
        let minecraft_eject_target: Option<(String, common::Coordinate)> = match &request.game {
            GameAudioContext::Minecraft(ctx) => {
                Some((ctx.world_uuid.clone(), ctx.coordinates.clone()))
            }
            GameAudioContext::Hytale(_) => None,
        };
        let (synthetic_player, position) = match request.game {
            GameAudioContext::Minecraft(ctx) => {
                let coordinates = ctx.coordinates.clone();
                (
                    PlayerEnum::Minecraft(MinecraftPlayer {
                        name: jukebox_name.clone(),
                        coordinates: ctx.coordinates,
                        orientation: Orientation { x: 0.0, y: 0.0 },
                        dimension: ctx.dimension,
                        deafen: false,
                        spectator: false,
                        world_uuid: Some(ctx.world_uuid),
                        alternative_identity: None,
                        player_uuid: None,
                    }),
                    coordinates,
                )
            }
            GameAudioContext::Hytale(_ctx) => {
                let coordinates = common::Coordinate { x: 0.0, y: 0.0, z: 0.0 };
                (
                    PlayerEnum::Hytale(HytalePlayer {
                        name: jukebox_name.clone(),
                        coordinates: coordinates.clone(),
                        orientation: Orientation { x: 0.0, y: 0.0 },
                        world_uuid: None,
                        dimension: Default::default(),
                        deafen: false,
                        spectator: false,
                        player_uuid: None,
                    }),
                    coordinates,
                )
            }
        };

        let cancel_token = self.parent_token.child_token();
        let cancel_token_clone = cancel_token.clone();

        let task = PlaybackTask::new(
            event_id.clone(),
            jukebox_name,
            position,
            frames,
            self.webhook_receiver.clone(),
            synthetic_player,
            cancel_token_clone,
        );

        let entry = PlaybackEntry {
            cancel_token: cancel_token.clone(),
            audio_file_id: audio_file_id.clone(),
            duration: Duration::from_millis(duration_ms),
        };
        self.active_playbacks
            .insert(event_id.clone(), entry)
            .await;
        self.dedup_cache
            .insert(dedup_key, event_id.clone())
            .await;

        let cleanup_cache = self.active_playbacks.clone();
        let cleanup_event_id = event_id.clone();

        tokio::spawn(async move {
            task.run().await;
            cleanup_cache.invalidate(&cleanup_event_id).await;
            tracing::info!(event_id = %cleanup_event_id, "Playback session cleaned up");
        });

        if let (Some(scheduler), Some((world_uuid, block_pos))) =
            (self.eject_scheduler.get(), minecraft_eject_target)
        {
            scheduler
                .schedule(
                    event_id.clone(),
                    world_uuid,
                    block_pos,
                    Duration::from_millis(duration_ms),
                )
                .await;
        }

        Ok(AudioEventResponse {
            event_id,
            duration_ms: duration_ms as u32,
        })
    }

    pub async fn stop_playback(&self, event_id: &str) -> Result<(), String> {
        if let Some(scheduler) = self.eject_scheduler.get() {
            scheduler.cancel(event_id).await;
        }
        if let Some(entry) = self.active_playbacks.get(event_id).await {
            entry.cancel_token.cancel();
            self.active_playbacks.invalidate(event_id).await;
            Ok(())
        } else {
            Err("Event not found or already stopped".to_string())
        }
    }

    pub fn stop_all(&self) {
        self.parent_token.cancel();
    }

    pub async fn is_file_playing(&self, audio_file_id: &str) -> bool {
        self.active_playbacks.run_pending_tasks().await;
        self.active_playbacks
            .iter()
            .any(|(_, entry)| entry.audio_file_id == audio_file_id)
    }
}
