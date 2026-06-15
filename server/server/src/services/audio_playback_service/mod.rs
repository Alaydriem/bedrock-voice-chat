mod eject_scheduler;
mod playback_entry;
mod playback_expiry;
mod parse_result;
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

use crate::relay::{AudioPeerQuery, AudioPuller};
use crate::stream::quic::WebhookReceiver;

use ogg_opus_parser::OggOpusParser;
use playback_entry::PlaybackEntry;
use playback_expiry::PlaybackExpiry;
use playback_task::PlaybackTask;

// How long the fulfiller waits for the first peer to answer an `AudioQuery`
// before giving up on a remote pull.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);

pub struct AudioPlaybackService {
    active_playbacks: Cache<String, PlaybackEntry>,
    dedup_cache: Cache<String, String>,
    // Parsed frames from a successful cross-server pull, keyed by audio_id. A
    // second remote-miss play of the same file reuses these and skips the
    // discover+pull+parse round trip entirely.
    remote_frame_cache: Cache<String, Arc<(Vec<Vec<u8>>, u64)>>,
    webhook_receiver: WebhookReceiver,
    audio_storage_path: String,
    parent_token: CancellationToken,
    eject_scheduler: OnceLock<Arc<EjectScheduler>>,
    // Cross-server jukebox discovery + pull. `None` when no relay client is
    // configured, in which case a local miss is a hard error (no peer to fetch
    // from).
    peer_query: Option<Arc<dyn AudioPeerQuery>>,
    audio_puller: Arc<dyn AudioPuller>,
}

impl AudioPlaybackService {
    pub fn new(
        webhook_receiver: WebhookReceiver,
        audio_storage_path: String,
        parent_token: CancellationToken,
        _max_concurrent_per_uuid: usize,
        peer_query: Option<Arc<dyn AudioPeerQuery>>,
        audio_puller: Arc<dyn AudioPuller>,
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
            remote_frame_cache: Cache::builder()
                .max_capacity(256)
                .time_to_live(Duration::from_secs(60))
                .build(),
            webhook_receiver,
            audio_storage_path,
            parent_token,
            eject_scheduler: OnceLock::new(),
            peer_query,
            audio_puller,
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
        let event_id = uuid::Uuid::now_v7().to_string();
        let cancel_token = self.parent_token.child_token();

        let local_frames = self.try_local_frames(conn, &audio_file_id).await?;
        let (frames, duration_ms) = match local_frames {
            Some(parsed) => parsed,
            None => {
                self.fetch_remote_frames(&event_id, &audio_file_id, &cancel_token)
                    .await?
            }
        };

        if frames.is_empty() {
            self.active_playbacks.invalidate(&event_id).await;
            return Err("No audio frames found in file".to_string());
        }

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
        let (synthetic_player, position, dimension) = Self::build_synthetic_player(&jukebox_name, request.game);

        let cancel_token_clone = cancel_token.clone();

        let task = PlaybackTask::new(
            event_id.clone(),
            jukebox_name,
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
                tracing::warn!(
                    event_id = %event_id,
                    "start_playback: eject_scheduler not wired; no auto-eject scheduled"
                );
            }
            (Some(_), None) => {
                tracing::debug!(
                    event_id = %event_id,
                    "start_playback: non-minecraft context; no auto-eject scheduled"
                );
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

    // Cross-server fetch on a local miss. Inserts a cancellable pending entry
    // FIRST (so `stop_playback`/shutdown can abort the fetch and `PlaybackExpiry`
    // cannot evict it mid-fetch — its placeholder TTL is the discovery timeout),
    // discovers a peer holding the file, HTTP-pulls the `.opus` into memory under
    // the cancellation token, and parses it. On timeout, no responder, cancel, or
    // pull/parse failure the pending entry is removed and an error is returned.
    async fn fetch_remote_frames(
        &self,
        event_id: &str,
        audio_file_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(Vec<Vec<u8>>, u64), String> {
        if let Some(cached) = self.remote_frame_cache.get(audio_file_id).await {
            return Ok((*cached).clone());
        }

        let peer_query = self
            .peer_query
            .as_ref()
            .ok_or_else(|| "Audio file not found and no relay configured".to_string())?;

        let pending = PlaybackEntry {
            cancel_token: cancel_token.clone(),
            audio_file_id: audio_file_id.to_string(),
            duration: DISCOVERY_TIMEOUT,
        };
        self.active_playbacks
            .insert(event_id.to_string(), pending)
            .await;

        let result = self
            .discover_and_pull(peer_query.clone(), event_id, audio_file_id, cancel_token)
            .await;

        match result {
            Ok(parsed) => {
                self.remote_frame_cache
                    .insert(audio_file_id.to_string(), Arc::new(parsed.clone()))
                    .await;
                Ok(parsed)
            }
            Err(e) => {
                self.active_playbacks.invalidate(event_id).await;
                Err(e)
            }
        }
    }

    // Discovery + pull + parse, racing every await against the cancellation token
    // so a `stop_playback`/shutdown aborts promptly and emits no frames.
    async fn discover_and_pull(
        &self,
        peer_query: Arc<dyn AudioPeerQuery>,
        event_id: &str,
        audio_file_id: &str,
        cancel_token: &CancellationToken,
    ) -> Result<(Vec<Vec<u8>>, u64), String> {
        let rx = peer_query.query_audio(audio_file_id, event_id);

        let resolved = tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err("Playback cancelled during discovery".to_string());
            }
            timed = tokio::time::timeout(DISCOVERY_TIMEOUT, rx) => {
                match timed {
                    Ok(Ok(resolved)) => resolved,
                    Ok(Err(_)) => return Err("Discovery channel closed".to_string()),
                    Err(_) => return Err("No peer responded with the audio file".to_string()),
                }
            }
        };

        let bytes = tokio::select! {
            _ = cancel_token.cancelled() => {
                return Err("Playback cancelled during fetch".to_string());
            }
            pulled = self.audio_puller.pull(
                &resolved.responder.host,
                resolved.responder.port,
                &resolved.available.stream_token,
            ) => {
                pulled.map_err(|e| format!("Audio pull failed: {}", e))?
            }
        };

        tokio::task::spawn_blocking(move || OggOpusParser::parse_frames_bytes(&bytes))
            .await
            .map_err(|e| format!("Task join error: {}", e))?
            .map_err(|e| format!("Ogg parsing error: {}", e))
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
                    }),
                    coordinates,
                    dimension,
                )
            }
            GameAudioContext::Hytale(_ctx) => {
                let coordinates = common::Coordinate { x: 0.0, y: 0.0, z: 0.0 };
                (
                    PlayerEnum::Hytale(HytalePlayer {
                        name: jukebox_name.to_string(),
                        coordinates: coordinates.clone(),
                        orientation: Orientation { x: 0.0, y: 0.0 },
                        world_uuid: None,
                        dimension: Default::default(),
                        deafen: false,
                        spectator: false,
                        player_uuid: None,
                    }),
                    coordinates,
                    Default::default(),
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
    use common::structs::packet::PacketType;
    use common::structs::relay::{AudioAvailable, RelayEndpoint};
    use crate::relay::ResolvedAudio;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;

    fn fixture_bytes() -> Vec<u8> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/audio/019d1701-6f1c-7661-ac44-4302ad6ba2f9.opus"
        );
        std::fs::read(path).expect("fixture read failed")
    }

    // Resolves a query immediately with a canned `AudioAvailable` + endpoint.
    struct StubPeerQuery {
        available: AudioAvailable,
        responder: RelayEndpoint,
        queries: Arc<AtomicUsize>,
    }

    impl AudioPeerQuery for StubPeerQuery {
        fn query_audio(&self, _audio_id: &str, _correlation_id: &str) -> oneshot::Receiver<ResolvedAudio> {
            self.queries.fetch_add(1, AtomicOrdering::SeqCst);
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(ResolvedAudio {
                available: self.available.clone(),
                responder: self.responder.clone(),
            });
            rx
        }
    }

    // Never resolves a query — exercises the discovery timeout path. The sender
    // is parked so the receiver stays open (it never errors), forcing the caller
    // to hit the discovery timeout rather than a closed channel.
    struct SilentPeerQuery {
        parked: std::sync::Mutex<Vec<oneshot::Sender<ResolvedAudio>>>,
    }

    impl AudioPeerQuery for SilentPeerQuery {
        fn query_audio(&self, _audio_id: &str, _correlation_id: &str) -> oneshot::Receiver<ResolvedAudio> {
            let (tx, rx) = oneshot::channel();
            self.parked.lock().expect("parked poisoned").push(tx);
            rx
        }
    }

    // Yields canned bytes on pull.
    struct StubPuller {
        bytes: Vec<u8>,
    }

    #[async_trait::async_trait]
    impl AudioPuller for StubPuller {
        async fn pull(&self, _host: &str, _port: u16, _token: &str) -> Result<Vec<u8>, anyhow::Error> {
            Ok(self.bytes.clone())
        }
    }

    // Blocks until the cancellation it shares is fired (or a long sleep elapses),
    // modeling a pull stalled in mid-transfer so the cancel race can be observed.
    struct StallingPuller {
        cancel: CancellationToken,
    }

    #[async_trait::async_trait]
    impl AudioPuller for StallingPuller {
        async fn pull(&self, _host: &str, _port: u16, _token: &str) -> Result<Vec<u8>, anyhow::Error> {
            tokio::select! {
                _ = self.cancel.cancelled() => Err(anyhow::anyhow!("aborted")),
                _ = tokio::time::sleep(Duration::from_secs(30)) => Ok(Vec::new()),
            }
        }
    }

    async fn empty_db() -> sea_orm::DatabaseConnection {
        use migration::{Migrator, MigratorTrait};
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        Migrator::up(&db, None).await.expect("apply migrations");
        db
    }

    fn minecraft_request(audio_id: &str) -> AudioPlayRequest {
        AudioPlayRequest {
            audio_file_id: audio_id.to_string(),
            game: GameAudioContext::Minecraft(MinecraftAudioContext {
                coordinates: common::Coordinate { x: 1.0, y: 2.0, z: 3.0 },
                dimension: Dimension::Overworld,
                world_uuid: "world-1".to_string(),
                relay_world_uuid: Some("W".to_string()),
            }),
        }
    }

    // A local miss with a peer that answers and a puller that returns valid
    // `.opus` bytes: discovery is issued, the pending entry is registered, the
    // file is pulled + parsed, and a relay-tagged playback runs (frames emitted).
    #[tokio::test]
    async fn miss_path_discovers_pulls_and_plays() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let queries = Arc::new(AtomicUsize::new(0));
        let peer_query = Arc::new(StubPeerQuery {
            available: AudioAvailable {
                audio_id: "audio-remote".into(),
                stream_token: "tok".into(),
                correlation_id: "corr".into(),
            },
            responder: RelayEndpoint {
                host: "peer".into(),
                port: 8443,
                primary: false,
            },
            queries: queries.clone(),
        });
        let puller = Arc::new(StubPuller {
            bytes: fixture_bytes(),
        });
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            CancellationToken::new(),
            1,
            Some(peer_query),
            puller,
        );
        let db = empty_db().await;

        let resp = service
            .start_playback(&db, minecraft_request("audio-remote"))
            .await
            .expect("remote miss should resolve to a playing event");

        assert_eq!(queries.load(AtomicOrdering::SeqCst), 1, "discovery issued once");
        assert!(service.active_playbacks.get(&resp.event_id).await.is_some());

        // The first emitted frame must be an AudioFrame whose synthetic sender is
        // tagged with the relay world so it fans out to peers.
        let packet = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("a playback frame should be emitted")
            .expect("channel open");
        assert_eq!(packet.packet_type, PacketType::AudioFrame);
        if let common::structs::packet::QuicNetworkPacketData::AudioFrame(frame) = packet.data {
            let sender = frame.sender.expect("synthetic sender present");
            let mc = sender.as_minecraft().expect("minecraft sender");
            assert_eq!(mc.relay_world_uuid, Some("W".to_string()));
        } else {
            panic!("expected AudioFrame data");
        }
    }

    // A second remote-miss play of the same audio_id reuses the cached parsed
    // frames: discovery/pull happens exactly once across both plays.
    #[tokio::test]
    async fn miss_path_second_play_uses_cache() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let queries = Arc::new(AtomicUsize::new(0));
        let peer_query = Arc::new(StubPeerQuery {
            available: AudioAvailable {
                audio_id: "audio-remote".into(),
                stream_token: "tok".into(),
                correlation_id: "corr".into(),
            },
            responder: RelayEndpoint {
                host: "peer".into(),
                port: 8443,
                primary: false,
            },
            queries: queries.clone(),
        });
        let puller = Arc::new(StubPuller {
            bytes: fixture_bytes(),
        });
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            CancellationToken::new(),
            1,
            Some(peer_query),
            puller,
        );
        let db = empty_db().await;

        service
            .start_playback(&db, minecraft_request("audio-remote"))
            .await
            .expect("first remote miss plays");

        // The dedup cache keys on coordinates + audio_id, so a same-coordinate
        // replay would be rejected as a duplicate. Move the second play to fresh
        // coordinates so it reaches the remote-miss path again.
        let mut second = minecraft_request("audio-remote");
        if let GameAudioContext::Minecraft(ctx) = &mut second.game {
            ctx.coordinates = common::Coordinate { x: 99.0, y: 99.0, z: 99.0 };
        }
        service
            .start_playback(&db, second)
            .await
            .expect("second remote miss plays from cache");

        assert_eq!(
            queries.load(AtomicOrdering::SeqCst),
            1,
            "discovery/pull must happen exactly once across both plays"
        );
    }

    // Cancelling the event token mid-fetch aborts the pull promptly, removes the
    // pending entry, returns an error, and emits no frames.
    #[tokio::test]
    async fn miss_path_cancel_mid_fetch_aborts() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let parent = CancellationToken::new();
        let peer_query = Arc::new(StubPeerQuery {
            available: AudioAvailable {
                audio_id: "audio-remote".into(),
                stream_token: "tok".into(),
                correlation_id: "corr".into(),
            },
            responder: RelayEndpoint {
                host: "peer".into(),
                port: 8443,
                primary: false,
            },
            queries: Arc::new(AtomicUsize::new(0)),
        });
        // The puller stalls on the same parent token so cancelling the parent (or
        // the event's child) aborts it.
        let puller = Arc::new(StallingPuller {
            cancel: parent.clone(),
        });
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            parent.clone(),
            1,
            Some(peer_query),
            puller,
        );
        let db = empty_db().await;

        let play = tokio::spawn({
            let service = Arc::new(service);
            let svc = service.clone();
            async move {
                let r = svc.start_playback(&db, minecraft_request("audio-remote")).await;
                (service, r)
            }
        });

        // Give discovery+pull a moment to enter the stall, then cancel everything.
        tokio::time::sleep(Duration::from_millis(50)).await;
        parent.cancel();

        let (service, result) = play.await.expect("task join");
        assert!(result.is_err(), "cancelled fetch must return an error");

        // No frames were emitted.
        assert!(rx.try_recv().is_err(), "no playback frames on a cancelled fetch");
        // The pending entry was removed.
        service.active_playbacks.run_pending_tasks().await;
        assert_eq!(service.active_playbacks.entry_count(), 0);
    }

    // No responder within the discovery timeout removes the pending entry and
    // returns an error without panicking. Waits out the real discovery timeout.
    #[tokio::test]
    async fn miss_path_no_responder_times_out() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            CancellationToken::new(),
            1,
            Some(Arc::new(SilentPeerQuery {
                parked: std::sync::Mutex::new(Vec::new()),
            })),
            Arc::new(StubPuller { bytes: Vec::new() }),
        );
        let db = empty_db().await;

        let result = service.start_playback(&db, minecraft_request("audio-remote")).await;
        assert!(result.is_err(), "no responder must error");

        service.active_playbacks.run_pending_tasks().await;
        assert_eq!(service.active_playbacks.entry_count(), 0);
    }

    // With no relay configured, a local miss is a hard error (no peer to fetch
    // from) rather than a panic.
    #[tokio::test]
    async fn miss_path_without_relay_errors() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            CancellationToken::new(),
            1,
            None,
            Arc::new(StubPuller { bytes: Vec::new() }),
        );
        let db = empty_db().await;
        let result = service.start_playback(&db, minecraft_request("audio-missing")).await;
        assert!(result.is_err());
    }

    // End-to-end pull: a real HTTPS endpoint serves the `.opus` and the
    // PRODUCTION `RelayAudioPuller` fetches it (not a stub), then playback runs.
    // Exercises the full discover -> HTTP pull -> parse -> play chain.
    #[tokio::test]
    async fn miss_path_pulls_over_http_end_to_end() {
        use crate::relay::RelayAudioPuller;

        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let port = spawn_opus_endpoint(fixture_bytes());

        let (tx, mut rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let peer_query = Arc::new(StubPeerQuery {
            available: AudioAvailable {
                audio_id: "audio-remote".into(),
                stream_token: "tok".into(),
                correlation_id: "corr".into(),
            },
            responder: RelayEndpoint {
                host: "127.0.0.1".into(),
                port,
                primary: false,
            },
            queries: Arc::new(AtomicUsize::new(0)),
        });
        let service = AudioPlaybackService::new(
            webhook,
            String::new(),
            CancellationToken::new(),
            1,
            Some(peer_query),
            RelayAudioPuller::new_shared(),
        );
        let db = empty_db().await;

        let resp = service
            .start_playback(&db, minecraft_request("audio-remote"))
            .await
            .expect("end-to-end pull should play");

        assert!(service.active_playbacks.get(&resp.event_id).await.is_some());
        let packet = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("a frame should be emitted")
            .expect("channel open");
        assert_eq!(packet.packet_type, PacketType::AudioFrame);
    }

    // Serves a single HTTPS request with a self-signed cert, replying with the
    // given `.opus` body on `GET /api/audio/stream?token=...`. Returns the bound
    // port. Mirrors the relaxed-TLS posture the production puller uses.
    fn spawn_opus_endpoint(body: Vec<u8>) -> u16 {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("generate self-signed cert");
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
        let key_der =
            rustls::pki_types::PrivateKeyDer::try_from(cert.signing_key.serialize_der())
                .expect("private key der");

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .expect("server tls config");
        let tls_config = Arc::new(tls_config);

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let port = listener.local_addr().expect("local addr").port();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut conn = rustls::ServerConnection::new(tls_config).expect("server connection");

            let mut request = Vec::new();
            loop {
                if conn.wants_read() {
                    if conn.read_tls(&mut stream).unwrap_or(0) == 0 {
                        break;
                    }
                    conn.process_new_packets().expect("process packets");
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = conn.reader().read(&mut buf) {
                        request.extend_from_slice(&buf[..n]);
                    }
                }
                if conn.wants_write() {
                    conn.write_tls(&mut stream).expect("write tls handshake");
                }
                if !conn.is_handshaking() && request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }

            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let mut response = header.into_bytes();
            response.extend_from_slice(&body);

            // Interleave plaintext writes with TLS flushes: the rustls write buffer
            // is bounded, so a large body must be drained onto the socket in chunks
            // rather than queued all at once.
            let mut offset = 0;
            while offset < response.len() {
                let written = conn
                    .writer()
                    .write(&response[offset..])
                    .expect("queue response chunk");
                offset += written;
                while conn.wants_write() {
                    conn.write_tls(&mut stream).expect("write tls response");
                }
            }
            conn.send_close_notify();
            let _ = conn.write_tls(&mut stream);
        });

        port
    }

    #[test]
    fn synthetic_player_carries_relay_world_uuid_from_context() {
        let ctx = MinecraftAudioContext {
            coordinates: common::Coordinate { x: 10.0, y: 64.0, z: -5.0 },
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
