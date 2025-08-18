use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use flume::Receiver;
use log::{info, warn};
use moka::sync::Cache;
use rodio::{mixer::Mixer, Sink, SpatialSink};
use tokio::task::JoinHandle;

use crate::audio::stream::jitter_buffer::{
    EncodedAudioFramePacket, JitterBuffer, SpatialAudioData,
};
use crate::audio::stream::stream_manager::audio_sink::AudioSink;
use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::Player;

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

    // Player data cache (listener information)
    players: Cache<String, Player>,
    current_player_name: String,
    player_gain_store: Arc<StdMutex<PlayerGainStore>>,

    // Per-player sinks and jitter buffer handles
    sinks: Cache<Vec<u8>, PlayerSinks>,

    // Audio mixer reference
    mixer: Arc<Mixer>,
}

impl SinkManager {
    pub fn new(
        consumer: Receiver<EncodedAudioFramePacket>,
        players: Cache<String, Player>,
        current_player_name: String,
        player_gain_store: Arc<StdMutex<PlayerGainStore>>,
        mixer: Arc<Mixer>,
    ) -> Self {
        Self {
            consumer: Some(consumer),
            shutdown: Arc::new(AtomicBool::new(false)),
            global_mute: Arc::new(AtomicBool::new(false)),
            players,
            current_player_name,
            player_gain_store,
            sinks: Cache::new(100),
            mixer,
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

    // Getter methods removed; updates applied per packet

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

        // Spawn an async task; use blocking recv() to wait indefinitely
        let handle = tokio::spawn(async move {
            while let Ok(packet) = consumer.recv() {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let author = packet.get_author();
                let author_bytes = packet.get_client_id();

                // Extract emitter data from packet
                let emitter_pos = packet.coordinate.clone();
                let emitter_spatial = packet.spatial.unwrap_or(false);

                // Get listener data from player cache (single read)
                let listener_info = players
                    .get(&current_player_name)
                    .map(|player| (player.coordinates.clone(), player.orientation.clone()));

                // Determine sink type based on spatial flag and listener availability
                let use_spatial =
                    emitter_spatial && listener_info.is_some() && emitter_pos.is_some();

                // Lookup per-player gain
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

                // Get or create PlayerSinks entry
                let mut bundle = sinks.get(&author_bytes).unwrap_or_else(|| {
                    let b = PlayerSinks::default();
                    sinks.insert(author_bytes.clone(), b.clone());
                    b
                });

                if use_spatial {
                    // Ensure spatial sink exists
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
                    let spatial_data: SpatialAudioData =
                        JitterBuffer::calculate_virtual_listener_audio_data(
                            &emitter_coordinate,
                            false,
                            &listener_coordinate,
                            &listener_orientation,
                        );

                    // Apply spatial positions and per-packet volume
                    if let Some(spatial_sink) = &bundle.spatial {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume = spatial_data.gain * gain_settings.gain * mute_mult;
                        spatial_sink.update_spatial_position(
                            [
                                emitter_coordinate.x,
                                emitter_coordinate.y,
                                emitter_coordinate.z,
                            ],
                            [
                                spatial_data.left_ear.x,
                                spatial_data.left_ear.y,
                                spatial_data.left_ear.z,
                            ],
                            [
                                spatial_data.right_ear.x,
                                spatial_data.right_ear.y,
                                spatial_data.right_ear.z,
                            ],
                            volume,
                        );
                    }

                    // Ensure jitter buffer exists and append source once
                    if bundle.spatial_handle.is_none() {
                        match JitterBuffer::new(packet.clone(), 120) {
                            Ok((source, handle)) => {
                                if let Some(spatial_sink) = &bundle.spatial {
                                    spatial_sink.append(source);
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
                    // Normal routing
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
                            [0.0, 0.0, 0.0],
                            [0.0, 0.0, 0.0],
                            [0.0, 0.0, 0.0],
                            volume,
                        );
                    }

                    if bundle.normal_handle.is_none() {
                        match JitterBuffer::new(packet.clone(), 120) {
                            Ok((source, handle)) => {
                                if let Some(normal_sink) = &bundle.normal {
                                    normal_sink.append(source);
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

                // Write back updated bundle
                sinks.insert(author_bytes.clone(), bundle);
            }
        });

        Ok(handle)
    }

    pub async fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Give existing jobs 500ms to clear
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

    // Legacy helpers removed; spatial updates are applied per-packet
}
