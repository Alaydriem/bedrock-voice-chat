mod eject_scheduler;
pub(crate) mod ogg_opus_parser;
mod parse_result;
mod playback_entry;
mod playback_expiry;
mod playback_task;
pub(crate) mod speaker_entry;
mod speaker_expiry;

pub use eject_scheduler::EjectScheduler;

use common::curia;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use common::players::MinecraftPlayer;
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
pub(crate) use speaker_entry::SpeakerEntry;
use speaker_expiry::SpeakerExpiry;

pub struct AudioPlaybackService {
    active_playbacks: Cache<String, PlaybackEntry>,
    // The synthetic player behind each live playback, keyed on the jukebox name the envelope
    // carries. Its own cache rather than a field on `PlaybackEntry` because the two are keyed
    // differently, with the same duration-derived expiry so a speaker cannot outlive or
    // predecease its playback.
    speakers: Arc<Cache<String, SpeakerEntry>>,
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
            speakers: Arc::new(
                Cache::builder()
                    .max_capacity(10000)
                    .expire_after(SpeakerExpiry)
                    .build(),
            ),
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
                ctx.world_uuid,
                ctx.coordinates.x,
                ctx.coordinates.y,
                ctx.coordinates.z,
                request.audio_file_id
            ),
        };

        if let Some(existing_event_id) = self.dedup_cache.get(&dedup_key).await {
            if self
                .active_playbacks
                .get(&existing_event_id)
                .await
                .is_some()
            {
                return Err("Duplicate play request".to_string());
            }
        }

        let audio_file_id = request.audio_file_id.clone();
        let event_id = uuid::Uuid::now_v7().to_string();
        let cancel_token = self.parent_token.child_token();

        let local_frames = self.try_local_frames(conn, &audio_file_id).await?;
        let (frames, duration_ms) = local_frames
            .ok_or_else(|| "Audio file not found".to_string())?;

        if frames.is_empty() {
            self.active_playbacks.invalidate(&event_id).await;
            return Err("No audio frames found in file".to_string());
        }

        curia::info!("Starting audio playback", { "event_id": event_id.to_string(), "file_id": audio_file_id.to_string(), "duration_ms": duration_ms, "frames": frames.len() });

        let stripped = event_id.replace('-', "");
        let jukebox_hash = &stripped[stripped.len() - 8..];
        let jukebox_name = format!(
            "{}{}",
            common::consts::audio::JUKEBOX_PLAYER_PREFIX,
            jukebox_hash
        );
        let minecraft_eject_target: Option<(String, common::Coordinate)> = match &request.game {
            GameAudioContext::Minecraft(ctx) => {
                Some((ctx.world_uuid.clone(), ctx.coordinates.clone()))
            }
        };
        let (synthetic_player, position, dimension) =
            Self::build_synthetic_player(&jukebox_name, request.game);

        // Published before the task is spawned rather than alongside it, so the first frame
        // cannot race the registration and route with no position.
        self.register_speaker(
            jukebox_name.clone(),
            synthetic_player.clone(),
            Duration::from_millis(duration_ms),
        )
        .await;

        let cancel_token_clone = cancel_token.clone();

        let task = PlaybackTask::new(
            event_id.clone(),
            jukebox_name.clone(),
            position,
            dimension,
            frames,
            self.webhook_receiver.clone(),
            synthetic_player,
            cancel_token_clone,
        );

        let entry = PlaybackEntry {
            cancel_token: cancel_token.clone(),
            audio_file_id: audio_file_id.clone(),
            duration: Duration::from_millis(duration_ms),
            jukebox_name: jukebox_name.clone(),
        };
        let entry_jukebox_name = jukebox_name;
        self.active_playbacks.insert(event_id.clone(), entry).await;
        self.dedup_cache.insert(dedup_key, event_id.clone()).await;

        let cleanup_cache = self.active_playbacks.clone();
        let cleanup_event_id = event_id.clone();
        let cleanup_speakers = self.speakers.clone();
        let cleanup_jukebox_name = entry_jukebox_name;

        tokio::spawn(async move {
            task.run().await;
            cleanup_cache.invalidate(&cleanup_event_id).await;
            cleanup_speakers.invalidate(&cleanup_jukebox_name).await;
            curia::info!("Playback session cleaned up", { "event_id": cleanup_event_id.to_string() });
        });

        match (self.eject_scheduler.get(), minecraft_eject_target) {
            (Some(scheduler), Some((world_uuid, block_pos))) => {
                scheduler
                    .schedule(
                        event_id.clone(),
                        world_uuid,
                        block_pos,
                        Duration::from_millis(duration_ms),
                    )
                    .await;
            }
            (None, _) => {
                curia::warn!("start_playback: eject_scheduler not wired; no auto-eject scheduled", { "event_id": event_id.to_string() });
            }
            (Some(_), None) => {
                curia::debug!("start_playback: non-minecraft context; no auto-eject scheduled", { "event_id": event_id.to_string() });
            }
        }

        Ok(AudioEventResponse {
            event_id,
            duration_ms: duration_ms as u32,
        })
    }

    // Parses frames from the local `.opus` when this server holds it. Returns
    // `None` (a local miss) when the `audio_file` row is absent/deleted or the
    // file is not on disk, so the caller can fall back to a cross-server fetch.
    async fn try_local_frames<C: ConnectionTrait>(
        &self,
        conn: &C,
        audio_file_id: &str,
    ) -> Result<Option<(Vec<Vec<u8>>, u64)>, String> {
        let row = audio_file::Entity::find_by_id(audio_file_id.to_string())
            .filter(audio_file::Column::Deleted.eq(0))
            .one(conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?;
        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let file_path = format!("{}/{}.opus", self.audio_storage_path, row.id);
        if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
            return Ok(None);
        }

        let parsed = tokio::task::spawn_blocking(move || OggOpusParser::parse_frames(&file_path))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Ogg parsing error: {}", e))?;
        Ok(Some(parsed))
    }

    pub async fn stop_playback(&self, event_id: &str) -> Result<(), String> {
        if let Some(scheduler) = self.eject_scheduler.get() {
            scheduler.cancel(event_id).await;
        }
        if let Some(entry) = self.active_playbacks.get(event_id).await {
            entry.cancel_token.cancel();
            self.forget_speaker(&entry.jukebox_name).await;
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

    /// Publishes a playback's speaker so audio routing can resolve its position.
    ///
    /// Called before the playback task is spawned, not alongside it: the first frame must not
    /// race the registration, or it routes with no position and is dropped.
    pub async fn register_speaker(
        &self,
        jukebox_name: String,
        player: PlayerEnum,
        duration: Duration,
    ) {
        self.speakers
            .insert(jukebox_name, SpeakerEntry { player, duration })
            .await;
    }

    /// Drops a playback's speaker. Called wherever the playback itself is invalidated.
    pub async fn forget_speaker(&self, jukebox_name: &str) {
        self.speakers.invalidate(jukebox_name).await;
    }

    /// The player behind a server-injected speaker, by the name its envelope carries.
    ///
    /// `None` once the playback has ended, which is what stops routing placing audio at a
    /// block nothing is playing from.
    pub async fn speaker_for(&self, service_name: &str) -> Option<PlayerEnum> {
        self.speakers
            .get(service_name)
            .await
            .map(|entry| entry.player)
    }

    /// A shared read handle for the audio path, which resolves a speaker per frame.
    ///
    /// Mirrors `PlayerCache::inner_arc`: the consumer gets the cache rather than a reference to
    /// this whole service, so nothing on the hot path depends on it.
    pub(crate) fn speakers(&self) -> Arc<Cache<String, SpeakerEntry>> {
        self.speakers.clone()
    }

    fn build_synthetic_player(
        jukebox_name: &str,
        game: GameAudioContext,
    ) -> (PlayerEnum, common::Coordinate, common::game_data::Dimension) {
        match game {
            GameAudioContext::Minecraft(ctx) => {
                let coordinates = ctx.coordinates.clone();
                let dimension = ctx.dimension.clone();
                (
                    PlayerEnum::Minecraft(MinecraftPlayer {
                        name: jukebox_name.to_string(),
                        coordinates: ctx.coordinates,
                        orientation: Orientation { x: 0.0, y: 0.0 },
                        dimension: ctx.dimension,
                        deafen: false,
                        spectator: false,
                        world_uuid: Some(ctx.world_uuid),
                        alternative_identity: None,
                        player_uuid: None,
                        relay_world_uuid: ctx.relay_world_uuid.clone(),
                        bridged_voice: false,
                    }),
                    coordinates,
                    dimension,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::game_data::Dimension;
    use common::request::audio::play::MinecraftAudioContext;

    #[test]
    fn synthetic_player_carries_relay_world_uuid_from_context() {
        let ctx = MinecraftAudioContext {
            coordinates: common::Coordinate {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            dimension: Dimension::Overworld,
            world_uuid: "world-abc".to_string(),
            relay_world_uuid: Some("W".to_string()),
        };

        let (player, _, _) = AudioPlaybackService::build_synthetic_player(
            "jkbx_test",
            GameAudioContext::Minecraft(ctx),
        );

        match player {
            PlayerEnum::Minecraft(mp) => {
                assert_eq!(mp.relay_world_uuid, Some("W".to_string()));
            }
            _ => panic!("expected Minecraft player"),
        }
    }
}
