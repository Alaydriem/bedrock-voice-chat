use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use flume::Receiver;
use log::{info, warn};
use moka::sync::Cache;
use rodio::{Player, Source, mixer::Mixer};
use std::num::NonZero;
use tauri::Emitter;
use tokio::task::JoinHandle;

use crate::audio::recording::RecordingProducer;
use crate::audio::stream::ActivityUpdate;
use crate::audio::stream::jitter_buffer::{EncodedAudioFramePacket, JitterBuffer, PanState};
use crate::audio::stream::stream_manager::audio_sink::AudioSink;
use crate::audio::stream::stream_manager::mono_to_panned::MonoToPanned;
use common::PlayerEnum;
use common::structs::SpatialAudioConfig;
use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::traits::player_data::PlayerData;

// Negate pan on platforms where the audio backend outputs channels
// in the opposite order to what we expect ([R,L] instead of [L,R]).
// Test each platform and flip the sign here if panning is inverted.
fn platform_adjusted_pan(pan: f32) -> f32 {
    pan
}

/// Converts a mono Source to stereo by duplicating each sample to both L and R channels
struct MonoToStereo<S>
where
    S: Source,
{
    inner: S,
    pending_sample: Option<f32>,
}

impl<S> MonoToStereo<S>
where
    S: Source,
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
    S: Source,
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
    S: Source,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len().map(|len| len * 2)
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

/// Convert a linear slider position (0.0-1.5) to a perceptually-correct amplitude factor.
/// Uses a power curve (x^2.5) so equal slider increments produce roughly equal loudness changes.
fn perceptual_gain(linear_position: f32) -> f32 {
    linear_position.powf(2.5)
}

#[derive(Clone, Default)]
struct PlayerSinks {
    normal: Option<Arc<AudioSink>>,
    spatial: Option<Arc<AudioSink>>,
    normal_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
    spatial_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
    spatial_pan_state: Option<Arc<PanState>>,
}

pub struct SinkManager {
    consumer: Option<Receiver<EncodedAudioFramePacket>>,
    shutdown: Arc<AtomicBool>,
    global_mute: Arc<AtomicBool>,
    panning_intensity: Arc<AtomicU32>,
    players: Cache<String, PlayerEnum>,
    current_player_name: String,
    player_gain_store: Arc<StdMutex<PlayerGainStore>>,
    sinks: Cache<Vec<u8>, PlayerSinks>,
    mixer: Arc<Mixer>,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    recording_producer: Option<RecordingProducer>,
    recording_active: Option<Arc<AtomicBool>>,
    spatial_config: SpatialAudioConfig,
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
        recording_active: Option<Arc<AtomicBool>>,
        spatial_config: SpatialAudioConfig,
        panning_intensity: f32,
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
            panning_intensity: Arc::new(AtomicU32::new(
                panning_intensity.clamp(0.0, 1.0).to_bits(),
            )),
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
            recording_active,
            spatial_config,
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

    pub fn update_panning_intensity(&self, intensity: f32) {
        self.panning_intensity
            .store(intensity.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
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
        let panning_intensity = self.panning_intensity.clone();
        let activity_tx = self.activity_tx.clone();
        let recording_producer = self.recording_producer.clone();
        let recording_active = self.recording_active.clone();
        let spatial_config = self.spatial_config.clone();

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

                let emitter_pos = packet
                    .emitter
                    .player_data
                    .as_ref()
                    .map(|p| p.get_position().clone());
                let deafen_emitter = packet
                    .emitter
                    .player_data
                    .as_ref()
                    .map(|p| p.is_deafened())
                    .unwrap_or(false);
                let emitter_spatial = packet.emitter.spatial.unwrap_or(true);

                let listener_info = players.get(&current_player_name).map(|player| {
                    let pos = player.get_position().clone();
                    let orient = player.get_orientation().clone();
                    (pos, orient)
                });

                if listener_info.is_none() {
                    log::debug!(
                        "Listener '{}' not found in player cache (cache size: {})",
                        current_player_name,
                        players.entry_count(),
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
                        let rodio_sink = Arc::new(Player::connect_new(&mixer));
                        let sink = Arc::new(AudioSink::Spatial(rodio_sink));
                        sink.play();
                        bundle.spatial = Some(sink);
                        bundle.spatial_pan_state = Some(Arc::new(PanState::new()));
                    }

                    let (listener_coordinate, listener_orientation) = listener_info.unwrap();
                    let emitter_coordinate = emitter_pos.unwrap();

                    let listener_player = players.get(&current_player_name);
                    let game = listener_player
                        .as_ref()
                        .map(|p| p.get_game())
                        .unwrap_or(common::Game::Minecraft);

                    let spatial_data = JitterBuffer::calculate_spatial_audio_data(
                        &emitter_coordinate,
                        deafen_emitter,
                        &listener_coordinate,
                        &listener_orientation,
                        game,
                        &spatial_config,
                    );

                    if let Some(pan_state) = &bundle.spatial_pan_state {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume =
                            spatial_data.volume * perceptual_gain(gain_settings.gain) * mute_mult;

                        let intensity = f32::from_bits(panning_intensity.load(Ordering::Relaxed));
                        let scaled_pan =
                            platform_adjusted_pan((spatial_data.pan * intensity).clamp(-1.0, 1.0));
                        let left = ((1.0 + scaled_pan) / 2.0).sqrt();
                        let right = ((1.0 - scaled_pan) / 2.0).sqrt();
                        pan_state.update(left, right, volume);
                    }

                    if bundle.spatial_handle.is_none() {
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("spatial_{}", author),
                            display_name.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_active.clone(),
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let (Some(spatial_sink), Some(pan_state)) =
                                    (&bundle.spatial, &bundle.spatial_pan_state)
                                {
                                    let panned_source =
                                        MonoToPanned::new(jitter_buffer, pan_state.clone());
                                    spatial_sink.append(panned_source);
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
                    } else if let Some(handle) = &bundle.spatial_handle {
                        let _ = handle.enqueue(packet.clone());
                    }
                } else {
                    if bundle.normal.is_none() {
                        let rodio_sink = Arc::new(Player::connect_new(&mixer));
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
                        let volume = 1.3 * perceptual_gain(gain_settings.gain) * mute_mult;
                        normal_sink.set_volume(volume);
                    }

                    if bundle.normal_handle.is_none() {
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("normal_{}", author),
                            display_name.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_active.clone(),
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let Some(normal_sink) = &bundle.normal {
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
                    } else if let Some(handle) = &bundle.normal_handle {
                        let _ = handle.enqueue(packet.clone());
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
