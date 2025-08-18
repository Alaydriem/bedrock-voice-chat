use moka::sync::Cache;
use rodio::{Sink, SpatialSink};
use rodio::mixer::Mixer;
use std::sync::Arc;
use std::time::Duration;
use log::{info, warn, debug};
use common::Player;
use common::structs::audio::PlayerGainStore;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::task::JoinHandle;
use tokio::sync::Mutex as AsyncMutex;
use std::collections::HashMap;
use crate::audio::stream::jitter_buffer::{EncodedAudioFramePacket, JitterBuffer, JitterBufferError, SpatialAudioData};
use super::audio_sink::{AudioSink, AudioSinkType, AudioSinkTarget};
use common::{Coordinate, Orientation};

#[derive(Clone)]
#[derive(Clone)]
struct SinkInfo {
    sink: Arc<AudioSink>,
    emitter_position: Option<Coordinate>,
    sink_type: AudioSinkType,
}

pub(crate) struct SinkManager {
    // Per-player audio sinks with emitter position tracking
    sinks: Cache<Vec<u8>, SinkInfo>,
    jitter_buffers: HashMap<Vec<u8>, Arc<AsyncMutex<JitterBuffer>>>,
    current_player_name: String,
    #[allow(unused)]
    app_handle: AppHandle,
    mixer: Arc<Mixer>,
    players: Arc<moka::sync::Cache<String, Player>>,
    player_gain_store: Arc<AsyncMutex<PlayerGainStore>>,
    consumer: Option<flume::Receiver<EncodedAudioFramePacket>>,
    shutdown: Arc<AtomicBool>,
    player_store_update_available: Arc<AtomicBool>
}

impl SinkManager {
    pub fn new(
        app_handle: AppHandle,
        mixer: Arc<Mixer>,
        players: Arc<moka::sync::Cache<String, Player>>,
        player_gain_store: PlayerGainStore,
        current_player_name: String,
        consumer: flume::Receiver<EncodedAudioFramePacket>,
    ) -> Self {
        Self {
            sinks: Cache::builder()
                .time_to_idle(Duration::from_secs(15 * 60))
                .build(),
            jitter_buffers: HashMap::new(),
            current_player_name,
            app_handle,
            mixer,
            players,
            player_gain_store: Arc::new(AsyncMutex::new(player_gain_store)),
            consumer: Some(consumer),
            shutdown: Arc::new(AtomicBool::new(false)),
            player_store_update_available: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update_player_store(&mut self, player_gain_store: PlayerGainStore) {
        // We'll update this asynchronously in a proper implementation
        _ = self.player_store_update_available.store(true, Ordering::Relaxed);
        // TODO: Update the async mutex
    }

    pub async fn listen(&mut self) -> Result<JoinHandle<()>, anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let shutdown = self.shutdown.clone();
        let consumer = self.consumer.take().ok_or_else(|| anyhow::anyhow!("SinkManager listener already started"))?;
        let players = self.players.clone();
        let current_player_name = self.current_player_name.clone();
        let player_gain_store = self.player_gain_store.clone();
        let sinks = self.sinks.clone();
        let mut jitter_buffers = HashMap::new();

        let handle = tokio::spawn(async move {
            while let packet = consumer.recv_async().await {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match packet {
                    Ok(packet) => {
                        let author = packet.get_author();
                        let author_bytes = packet.get_client_id();

                        // Extract emitter data from packet
                        let emitter_pos = packet.coordinate.clone();
                        let emitter_spatial = packet.spatial.unwrap_or(false);

                        // Get listener data from player cache (single read)
                        let listener_info = if let Some(player) = players.get(&current_player_name) {
                            Some((player.coordinates.clone(), player.orientation.clone()))
                        } else {
                            None
                        };

                        // Get or create JitterBuffer for this player
                        let jitter_buffer = match jitter_buffers.get_mut(&author_bytes) {
                            Some(buffer) => buffer,
                            None => {
                                // Create new JitterBuffer for this player
                                match JitterBuffer::new() {
                                    Ok(buffer) => {
                                        info!("Created new JitterBuffer for player: {}", author);
                                        jitter_buffers.insert(author_bytes.clone(), Arc::new(AsyncMutex::new(buffer)));
                                        jitter_buffers.get_mut(&author_bytes).unwrap()
                                    }
                                    Err(e) => {
                                        warn!("Failed to create JitterBuffer for player {}: {:?}", author, e);
                                        continue;
                                    }
                                }
                            }
                        };

                        // Enqueue packet to JitterBuffer
                        {
                            let mut buffer = jitter_buffer.lock().await;
                            if let Err(e) = buffer.enqueue(packet.clone()) {
                                warn!("Failed to enqueue packet for player {}: {:?}", author, e);
                                continue;
                            }
                        }

                        // Determine sink type based on spatial flag and listener availability
                        let use_spatial = emitter_spatial && listener_info.is_some();
                        let sink_type = if use_spatial { AudioSinkType::Spatial } else { AudioSinkType::Normal };
                        
                        // Get or create AudioSink for this player/type combination
                        let mut sink_info = if let Some(info) = sinks.get(&author_bytes) {
                            // Update emitter position if this is spatial
                            if use_spatial {
                                let mut updated_info = info.clone();
                                updated_info.emitter_position = emitter_pos;
                                sinks.insert(author_bytes.clone(), updated_info.clone());
                                updated_info
                            } else {
                                info.clone()
                            }
                        } else {
                            // Create new AudioSink and SinkInfo
                            warn!("AudioSink creation not yet implemented for player: {}", author);
                            
                            // For now, just create a placeholder SinkInfo without actual sink
                            let sink_info = SinkInfo {
                                sink: Arc::new(AudioSink::new(
                                    AudioSinkTarget::Normal(Arc::new(Sink::connect_new(&rodio::mixer::Mixer::new()))), 
                                    opus::Decoder::new(48000, opus::Channels::Mono).unwrap(),
                                )),
                                emitter_position: emitter_pos,
                                sink_type,
                            };
                            
                            sinks.insert(author_bytes.clone(), sink_info.clone());
                            sink_info
                        };

                        // Handle spatial positioning if we have the required data
                        if use_spatial && listener_info.is_some() {
                            let emitter_coordinate = emitter_pos.unwrap();
                            let (listener_coordinate, listener_orientation) = listener_info.unwrap();

                            // Calculate spatial positioning
                            let spatial_data = JitterBuffer::calculate_virtual_listener_audio_data(
                                &emitter_coordinate,  // FROM packet (emitter)
                                false,                 // deafen logic handled via output.rs mute
                                &listener_coordinate,  // FROM player cache (listener)
                                &listener_orientation, // FROM player cache (listener)
                            );

                            debug!("Calculated spatial data for {}: gain={}, distance={}", 
                                author, spatial_data.gain, 
                                ((emitter_coordinate.x - listener_coordinate.x).powi(2) + 
                                 (emitter_coordinate.y - listener_coordinate.y).powi(2) + 
                                 (emitter_coordinate.z - listener_coordinate.z).powi(2)).sqrt());

                            // TODO: Apply spatial_data to AudioSink when proper sink creation is implemented
                            // sink_info.sink.update_spatial_position(
                            //     [emitter_coordinate.x, emitter_coordinate.y, emitter_coordinate.z],
                            //     [spatial_data.left_ear.x, spatial_data.left_ear.y, spatial_data.left_ear.z],
                            //     [spatial_data.right_ear.x, spatial_data.right_ear.y, spatial_data.right_ear.z],
                            //     spatial_data.gain
                            // );
                        }

                        // TODO: Create JitterBuffer source from packet and append to sink
                        // let jitter_buffer_source = JitterBuffer::from_packet(packet);
                        // sink_info.sink.append(jitter_buffer_source);
                            }

                            continue; // Skip to next packet until sink creation is implemented
                        };

                        // TODO: Connect JitterBuffer to AudioSink as Source
                        // let jitter_buffer_source = Arc::new(Mutex::new(jitter_buffer));
                        // sink_info.sink.append(jitter_buffer_source);
                    }
                    Err(_e) => {
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    pub async fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Give existing jobs 500ms to clear
        tokio::time::sleep(Duration::from_millis(500)).await;

        for (_, sink_info) in self.sinks.iter() {
            sink_info.sink.clear_and_stop();
        }

        info!("SinkManager has been stopped.");
    }

    /// Update spatial positioning for all active spatial sinks
    /// This should be called every frame for real-time spatial audio
    pub fn update_spatial_positioning(&self) {
        // Get current listener position
        let listener_info = if let Some(player) = self.players.get(&self.current_player_name) {
            (player.coordinates.clone(), player.orientation.clone())
        } else {
            return; // No listener position available
        };

        let (listener_coordinate, listener_orientation) = listener_info;

        // Update all spatial sinks
        for (sink_key, sink_info) in self.sinks.iter() {
            // Check if this is a spatial sink and has emitter position
            if sink_info.sink_type == AudioSinkType::Spatial {
                if let Some(emitter_coordinate) = &sink_info.emitter_position {
                    // Calculate spatial positioning
                    let spatial_data = JitterBuffer::calculate_virtual_listener_audio_data(
                        emitter_coordinate,
                        false, // deafen logic handled via output.rs mute
                        &listener_coordinate,
                        &listener_orientation,
                    );

                    // Apply spatial positioning to sink
                    sink_info.sink.update_spatial_position(
                        [emitter_coordinate.x, emitter_coordinate.y, emitter_coordinate.z],
                        [spatial_data.left_ear.x, spatial_data.left_ear.y, spatial_data.left_ear.z],
                        [spatial_data.right_ear.x, spatial_data.right_ear.y, spatial_data.right_ear.z],
                        spatial_data.gain
                    );

                    debug!("Updated spatial positioning for sink: gain={}, distance={}", 
                        spatial_data.gain,
                        ((emitter_coordinate.x - listener_coordinate.x).powi(2) + 
                         (emitter_coordinate.y - listener_coordinate.y).powi(2) + 
                         (emitter_coordinate.z - listener_coordinate.z).powi(2)).sqrt());
                }
            }
        }
    }

    pub fn get_or_create_sink(
        &self,
        player_id: Vec<u8>,
        sink_type: AudioSinkType,
    ) -> SinkInfo {
        self.sinks.get(&player_id).unwrap_or_else(|| {
            let new_sink = match sink_type {
                AudioSinkType::Normal => {
                    let rodio_sink = Arc::new(Sink::connect_new(&self.mixer));
                    AudioSink::new(
                        AudioSinkTarget::Normal(rodio_sink),
                        opus::Decoder::new(48000, opus::Channels::Mono).unwrap(),
                    )
                }
                AudioSinkType::Spatial => {
                    let rodio_sink = Arc::new(SpatialSink::connect_new(
                        &self.mixer,
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                    ));
                    AudioSink::new(
                        AudioSinkTarget::Spatial(rodio_sink),
                        opus::Decoder::new(48000, opus::Channels::Mono).unwrap(),
                    )
                }
            };
            
            let sink_info = SinkInfo {
                sink: Arc::new(new_sink),
                emitter_position: None,
                sink_type,
            };
            
            self.sinks.insert(player_id.clone(), sink_info.clone());
            sink_info
        })
    }
}
