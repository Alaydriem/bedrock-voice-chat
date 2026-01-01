use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use flume::Receiver;
use log::{info, warn};
use moka::sync::Cache;
use rodio::{mixer::Mixer, Sink, SpatialSink, Source};
use tauri::Emitter;
use tokio::task::JoinHandle;

use crate::audio::recording::RecordingProducer;
use crate::audio::stream::jitter_buffer::{
    EncodedAudioFramePacket, JitterBuffer, SpatialAudioData,
};
use crate::audio::stream::stream_manager::audio_sink::AudioSink;
use crate::audio::stream::ActivityUpdate;
use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::{Coordinate, PlayerEnum};
use common::traits::player_data::PlayerData;

/// Converts a mono Source to stereo by duplicating each sample to both L and R channels
struct MonoToStereo<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    pending_sample: Option<f32>,
}

impl<S> MonoToStereo<S>
where
    S: Source<Item = f32>,
{
    fn new(source: S) -> Self {
        Self {
            inner: source,
            pending_sample: None,
        }
    }
}

impl<S> Iterator for MonoToStereo<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have a pending sample, return it as the R channel
        if let Some(sample) = self.pending_sample.take() {
            return Some(sample);
        }

        // Get next sample from mono source
        if let Some(sample) = self.inner.next() {
            // Store it for R channel
            self.pending_sample = Some(sample);
            // Return it as L channel
            Some(sample)
        } else {
            None
        }
    }
}

impl<S> Source for MonoToStereo<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len().map(|len| len * 2)
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[derive(Clone, Default)]
struct PlayerSinks {
    normal: Option<Arc<AudioSink>>,
    spatial: Option<Arc<AudioSink>>,
    normal_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
    spatial_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
}

pub struct SinkManager {
    consumer: Option<Receiver<EncodedAudioFramePacket>>,
    shutdown: Arc<AtomicBool>,
    global_mute: Arc<AtomicBool>,
    players: Cache<String, PlayerEnum>,
    current_player_name: String,
    player_gain_store: Arc<StdMutex<PlayerGainStore>>,
    sinks: Cache<Vec<u8>, PlayerSinks>,
    mixer: Arc<Mixer>,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    // Recording support - captures audio at playback time for proper timecode alignment
    recording_producer: Option<RecordingProducer>,
    recording_enabled: Arc<AtomicBool>,
}

impl SinkManager {
    pub fn new(
        consumer: Receiver<EncodedAudioFramePacket>,
        players: Cache<String, PlayerEnum>,
        current_player_name: String,
        player_gain_store: Arc<StdMutex<PlayerGainStore>>,
        mixer: Arc<Mixer>,
        app_handle: tauri::AppHandle,
        recording_producer: Option<RecordingProducer>,
        recording_enabled: Arc<AtomicBool>,
    ) -> Self {
        // Create activity streaming channel
        let (activity_tx, activity_rx) = flume::unbounded::<ActivityUpdate>();

        // Spawn activity streaming task
        let app_handle_clone = app_handle.clone();
        tokio::spawn(async move {
            let mut batch_timer = tokio::time::interval(Duration::from_millis(100));
            let mut current_activities = std::collections::HashMap::new();

            loop {
                tokio::select! {
                    // Collect activity updates
                    Ok(update) = activity_rx.recv_async() => {
                        current_activities.insert(update.player_name.clone(), update.rms_level);
                    }

                    // Batch and stream every 100ms
                    _ = batch_timer.tick() => {
                        if !current_activities.is_empty() {
                            if let Err(e) = app_handle_clone.emit("audio-activity", &current_activities) {
                                log::warn!("Failed to emit audio activity: {}", e);
                            }
                            current_activities.clear(); // Reset for next batch
                        }
                    }
                }
            }
        });

        Self {
            consumer: Some(consumer),
            shutdown: Arc::new(AtomicBool::new(false)),
            global_mute: Arc::new(AtomicBool::new(false)),
            players,
            current_player_name,
            player_gain_store,
            sinks: Cache::builder()
                .time_to_live(Duration::from_secs(15 * 60)) // 15 minutes TTL
                .max_capacity(100)
                .build(),
            mixer,
            activity_tx: Some(activity_tx),
            app_handle,
            recording_producer,
            recording_enabled,
        }
    }

    pub fn update_player_store(&mut self, player_gain_store: PlayerGainStore) {
        if let Ok(mut guard) = self.player_gain_store.lock() {
            *guard = player_gain_store;
        }
    }

    pub fn update_global_mute(&self, muted: bool) {
        self.global_mute.store(muted, Ordering::Relaxed);
    }

    pub async fn listen(&mut self) -> Result<JoinHandle<()>, anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let shutdown = self.shutdown.clone();
        let consumer = self
            .consumer
            .take()
            .ok_or_else(|| anyhow::anyhow!("SinkManager listener already started"))?;
        let players = self.players.clone();
        let current_player_name = self.current_player_name.clone();
        let player_gain_store = self.player_gain_store.clone();
        let sinks = self.sinks.clone();
        let mixer = self.mixer.clone();
        let global_mute = self.global_mute.clone();
        let activity_tx = self.activity_tx.clone();
        let recording_producer = self.recording_producer.clone();
        let recording_enabled = self.recording_enabled.clone();

        // Spawn an async task; use async recv to avoid blocking
        let handle = tokio::spawn(async move {
            while let Ok(packet) = consumer.recv_async().await {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let author = packet.get_author();
                let author_bytes = packet.get_client_id();

                let display_name = match packet.emitter.name.clone() {
                    name if !name.is_empty() && name != "api" => name,
                    _ => author.clone(),
                };

                let emitter_pos = packet.emitter.player_data.as_ref().map(|p| {
                    use common::traits::player_data::PlayerData;
                    p.get_position().clone()
                });
                let emitter_spatial = packet.emitter.spatial.unwrap_or(false);

                let listener_info = players
                    .get(&current_player_name)
                    .map(|player| {
                        let pos = player.get_position().clone();
                        let orient = player.get_orientation().clone();
                        (pos, orient)
                    });

                if listener_info.is_none() {
                    let cached_players: Vec<String> = players.iter()
                        .map(|(k, _)| k.as_ref().clone())
                        .collect();
                    log::warn!(
                        "Listener '{}' not found in player cache (cache size: {}, cached players: {:?})",
                        current_player_name,
                        players.entry_count(),
                        cached_players
                    );
                }

                let use_spatial =
                    emitter_spatial && listener_info.is_some() && emitter_pos.is_some();

                let gain_settings: PlayerGainSettings = {
                    let store = player_gain_store.lock().ok();
                    store
                        .and_then(|s| s.0.get(&author).cloned())
                        .unwrap_or(PlayerGainSettings {
                            gain: 1.0,
                            muted: false,
                        })
                };
                if gain_settings.muted {
                    continue;
                }

                let mut bundle = sinks.get(&author_bytes).unwrap_or_else(|| {
                    let b = PlayerSinks::default();
                    if let Some(existing) = sinks.get(&author_bytes) {
                        existing
                    } else {
                        sinks.insert(author_bytes.clone(), b.clone());
                        b
                    }
                });

                if use_spatial {
                    if bundle.spatial.is_none() {
                        let rodio_sink = Arc::new(SpatialSink::connect_new(
                            &mixer,
                            [0.0, 0.0, 0.0],
                            [0.0, 0.0, 0.0],
                            [0.0, 0.0, 0.0],
                        ));
                        let sink = Arc::new(AudioSink::Spatial(rodio_sink));
                        sink.play();
                        bundle.spatial = Some(sink);
                    }

                    let (listener_coordinate, listener_orientation) = listener_info.unwrap();
                    let emitter_coordinate = emitter_pos.unwrap();

                    // Calculate actual distance for logging
                    let dx = emitter_coordinate.x - listener_coordinate.x;
                    let dy = emitter_coordinate.y - listener_coordinate.y;
                    let dz = emitter_coordinate.z - listener_coordinate.z;
                    let actual_distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    let spatial_data: SpatialAudioData =
                        JitterBuffer::calculate_virtual_listener_audio_data(
                            &emitter_coordinate,
                            false,
                            &listener_coordinate,
                            &listener_orientation,
                        );

                    if let Some(spatial_sink) = &bundle.spatial {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume = spatial_data.gain * gain_settings.gain * mute_mult;
                        spatial_sink.update_spatial_position(
                            &emitter_coordinate,
                            &spatial_data.left_ear,
                            &spatial_data.right_ear,
                            volume,
                        );
                    }

                    if bundle.spatial_handle.is_none() {
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("spatial_{}", author),
                            display_name.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_enabled.clone(),
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let Some(spatial_sink) = &bundle.spatial {
                                    // Convert mono jitter buffer to stereo for proper playback
                                    let stereo_source = MonoToStereo::new(jitter_buffer);
                                    spatial_sink.append(stereo_source);
                                }
                                bundle.spatial_handle = Some(handle.clone());
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create spatial jitter buffer for {}: {:?}",
                                    author, e
                                );
                                continue;
                            }
                        }
                    } else {
                        if let Some(handle) = &bundle.spatial_handle {
                            let _ = handle.enqueue(packet.clone(), Some(spatial_data));
                        }
                    }
                } else {
                    if bundle.normal.is_none() {
                        let rodio_sink = Arc::new(Sink::connect_new(&mixer));
                        let sink = Arc::new(AudioSink::Normal(rodio_sink));
                        sink.play();
                        bundle.normal = Some(sink);
                    }

                    if let Some(normal_sink) = &bundle.normal {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume = 1.3 * gain_settings.gain * mute_mult;
                        normal_sink.update_spatial_position(
                            &Coordinate::default(),
                            &Coordinate::default(),
                            &Coordinate::default(),
                            volume,
                        );
                    }

                    if bundle.normal_handle.is_none() {
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("normal_{}", author),
                            display_name.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_enabled.clone(),
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let Some(normal_sink) = &bundle.normal {
                                    // Convert mono jitter buffer to stereo for proper playback
                                    let stereo_source = MonoToStereo::new(jitter_buffer);
                                    normal_sink.append(stereo_source);
                                }
                                bundle.normal_handle = Some(handle.clone());
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create normal jitter buffer for {}: {:?}",
                                    author, e
                                );
                                continue;
                            }
                        }
                    } else {
                        if let Some(handle) = &bundle.normal_handle {
                            let _ = handle.enqueue(packet.clone(), None);
                        }
                    }
                }

                sinks.insert(author_bytes.clone(), bundle);
            }
        });

        Ok(handle)
    }

    pub async fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        tokio::time::sleep(Duration::from_millis(500)).await;

        for (_, bundle) in self.sinks.iter() {
            if let Some(h) = &bundle.normal_handle {
                h.stop();
            }
            if let Some(h) = &bundle.spatial_handle {
                h.stop();
            }
            if let Some(s) = &bundle.normal {
                s.clear_and_stop();
            }
            if let Some(s) = &bundle.spatial {
                s.clear_and_stop();
            }
        }

        info!("SinkManager has been stopped.");
    }
}
