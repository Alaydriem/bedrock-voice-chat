use crate::AudioPacket;
use crate::audio::types::{AudioDevice, BUFFER_SIZE};
use common::{
    structs::{
        audio::{
            PlayerGainSettings,
            PlayerGainStore
        },
        packet::{AudioFramePacket, PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket}
    }, Coordinate, Orientation, Player
};
use base64::{engine::general_purpose, Engine as _};
use log::{error, info, warn};
use rodio::{buffer::SamplesBuffer, Sink, SpatialSink, OutputStreamBuilder};
use std::{
    sync::{
        atomic::{
            AtomicBool,
            Ordering
        },
        mpsc::{
            self,
            Receiver,
            Sender
        },
        Arc,
    }, time::Duration
};
use tokio::task::{AbortHandle, JoinHandle};
use moka::future::Cache;
use anyhow::anyhow;
use tauri::async_runtime::Mutex;
use tauri::Emitter;
use once_cell::sync::Lazy;

static MUTE_OUTPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

static PLAYER_ATTENUATION_SETTINGS: Lazy<std::sync::Mutex<serde_json::Value>> = Lazy::new(|| {
    std::sync::Mutex::new(serde_json::to_value(PlayerGainStore::default()).expect("Failed to serialize PlayerGainStore"))
});

static UPDATE_PLAYER_ATTENUATION_SETTINGS: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

use super::sink_manager::SinkManager;

/// This is our decoded audio stream
#[derive(Debug, Clone)]
struct DecodedAudioFramePacket {
    pub owner: Option<PacketOwner>,
    pub buffer: SamplesBuffer,
    pub coordinate: Option<Coordinate>,
    #[allow(dead_code)]
    pub orientation: Option<Orientation>,
    pub spatial: Option<bool>
}

#[derive(Debug, Clone)]
struct SpatialAudioData {
    pub emitter: Coordinate,
    pub left_ear: Coordinate,
    pub right_ear: Coordinate,
    pub gain: f32,
}

impl DecodedAudioFramePacket {
    pub fn get_author(&self) -> String {
        match &self.owner {
            Some(owner) => {
                // Utilize the client ID so that the same author can receive and hear multiple incoming
                // network streams. Without this, the audio packets for the same author across two streams
                // come in sequence and playback sounds corrupted
                return general_purpose::STANDARD.encode(&owner.client_id);
            }
            None => String::from("")
        }
    }
}

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    players: Arc<moka::sync::Cache<String, Player>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<Cache<String, String>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    player_presence: Arc<moka::sync::Cache<String, bool>>
}

impl common::traits::StreamTrait for OutputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        match key.as_str() {
            "mute" => {
                self.mute();
            },
            "player_gain_store" => {
                match serde_json::from_str::<PlayerGainStore>(&value) {
                    Ok(settings) => {
                        let mut lock_settings = PLAYER_ATTENUATION_SETTINGS.lock().unwrap();
                        *lock_settings = serde_json::to_value(settings).expect("Failed to serialize PlayerGainStore");
                        UPDATE_PLAYER_ATTENUATION_SETTINGS.store(true, Ordering::Relaxed);
                        drop(lock_settings);
                    },
                    Err(e) => {
                        error!("Failed to parse PlayerGainStore: {:?}", e);
                    }
                };
                info!("Player gain store updated.");
            },
            "player_presence" => {
                if !self.player_presence.contains_key(&value) {
                    self.app_handle.emit(
                        crate::events::event::player_presence::PLAYER_PRESENCE,
                        crate::events::event::player_presence::Presence::new(
                            value.clone(),
                            "online".to_string()
                    )).unwrap();
                }
                
                self.player_presence.insert(value.clone(), true);
            },
            _ => self.metadata.insert(key.clone(), value.clone()).await
        };
        
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        // Give existing jobs 500ms to clear
        _ = tokio::time::sleep(Duration::from_millis(500)).await;

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        info!("Output stream has been stopped.");
        self.jobs = vec![];

        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        
        let mut jobs = vec![];
        let (producer, consumer) = mpsc::channel();

        // Playback the PCM data
        match self.playback(
            consumer,
            self.shutdown.clone(),
            self.metadata.clone(),
            self.players.clone(),
        ).await {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Listen to the network stream
        match self.listener(
            Arc::new(producer),
            self.shutdown.clone(),
            self.players.clone()
        ).await {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        self.jobs = jobs.iter().map(|handle| handle.abort_handle()).collect();
        
        Ok(())
    }
}

impl OutputStream {
    pub fn new(
        device: Option<AudioDevice>,
        bus: Arc<flume::Receiver<AudioPacket>>,
        metadata: Arc<moka::future::Cache<String, String>>,
        app_handle: tauri::AppHandle
    ) -> Self {
        let players = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(15 * 60))
            .build();

        Self {
            device,
            bus,
            players: Arc::new(players),
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone(),
            player_presence: Arc::new(moka::sync::Cache::builder()
                .time_to_idle(Duration::from_secs(3 * 60))
                .build())
        }
    }
    
    /// Listens to incoming network packet events from the server
    /// Translates them, then sends them to playback for processing
    async fn listener(
        &mut self,
        producer: Arc<Sender<DecodedAudioFramePacket>>,
        shutdown: Arc<AtomicBool>,
        players: Arc<moka::sync::Cache<String, Player>>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let device_config = rodio::cpal::StreamConfig {
                        channels: config.channels(),
                        sample_rate: config.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(crate::audio::types::BUFFER_SIZE),
                    };

                    let bus = self.bus.clone();

                    let handle = tokio::spawn(async move {
                        // Opus decoders are stored in a ttl-hashmap
                        // Opus streams maintain state between opus packets, so we need to re-use
                        // the same decode, per stream
                        // We can save some memory but automatically dropping unused decoders
                        // after 15 minutes
                        let decoders: Cache<Vec<u8>, Arc<Mutex<opus::Decoder>>> = Cache::builder()
                            .time_to_idle(Duration::from_secs(15 * 60))
                            .build();

                        #[allow(irrefutable_let_patterns)]
                        while let packet = bus.recv_async().await {
                            if shutdown.load(Ordering::Relaxed) {
                                warn!("Output listener handler stopped.");
                                break;
                            }

                            match packet {
                                Ok(packet) => {
                                    match packet.data.get_packet_type() {
                                        PacketType::AudioFrame => OutputStream::handle_audio_data(
                                            &decoders,
                                            &device_config,
                                            producer.clone(),
                                            &packet.data
                                        ).await,
                                        PacketType::PlayerData => OutputStream::handle_player_data(
                                            players.clone(),
                                            &packet.data
                                        ).await,
                                        _ => {}
                                    }
                                },
                                Err(e) => {
                                    warn!("Failed to receive packet: {:?}", e);
                                }
                            }
                        }
                    });

                    return Ok(handle);
                },
                Err(e) => return Err(e),
            },
            None => {
                return Err(anyhow!(
                    "Output Stream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }
    
    /// Handles playback of the PCM Audio Stream to the output device
    async fn playback(
        &mut self,
        consumer: Receiver<DecodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>,
        metadata: Arc<Cache<String, String>>,
        players: Arc<moka::sync::Cache<String, Player>>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        let current_player_name = match metadata.get("current_player").await {
            Some(name) => name,
            None => return Err(anyhow!("Playback stream cannot start without a player name set. Hint: .metadata('current_player', String) first."))
        };

        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let cpal_device: Option<rodio::cpal::Device> = device.clone().into();
                    let handle = match cpal_device {
                        Some(cpal_device) => tokio::spawn(async move {
                            log::info!("started receiving audio stream");
                            let builder = match OutputStreamBuilder::from_device(cpal_device) {
                                Ok(b) => b,
                                Err(e) => { error!("Could not create OutputStreamBuilder: {:?}", e); return; }
                            };
                            let stream_config: rodio::cpal::StreamConfig = config.clone().into();
                            let builder = builder.with_config(&stream_config);
                            let stream = match builder.open_stream_or_fallback() {
                                Ok(s) => s,
                                Err(e) => { error!("Could not acquire OutputStream. Try restarting the stream? {:?}", e); return; }
                            };

                            let mixer = stream.mixer();
                            let mut sink_manager = SinkManager::new(&mixer);
    
                            let mut player_gain_store_settings: PlayerGainStore = PlayerGainStore::default();
                            
                            // Iterate over the incoming PCM data
                            #[allow(irrefutable_let_patterns)]
                            while let packet = consumer.recv() {
                                if shutdown.load(Ordering::Relaxed) {
                                    for sink in sink_manager.sinks.iter() {
                                        sink.1.sink.clear();
                                        sink.1.spatial_sink.clear();
                                        sink.1.sink.stop();
                                        sink.1.spatial_sink.stop();
                                    }

                                    // We need to hard-drop the sink_manager so any audio in the existing buffer gets flushed
                                    drop(sink_manager);
                                    warn!("{} {} ended.", device.io.to_string(), device.display_name);
                                    break;
                                }

                                // If the player attentuation settings are updated, reparse them
                                if UPDATE_PLAYER_ATTENUATION_SETTINGS.load(Ordering::Relaxed) {
                                    let current_settings = PLAYER_ATTENUATION_SETTINGS.lock().unwrap();
                                    match serde_json::from_value::<PlayerGainStore>(current_settings.clone()) {
                                        Ok(settings) => {
                                            player_gain_store_settings = settings;
                                        },
                                        Err(e) => {
                                            error!("Failed to parse PlayerGainStore: {:?}", e);
                                        }
                                    };
                                    
                                    UPDATE_PLAYER_ATTENUATION_SETTINGS.store(false, Ordering::Relaxed);
                                    drop(current_settings)
                                }
    
                                match packet {
                                    Ok(packet) => {
                                        // Clients only hear audio that the server determins they can hear
                                        // However the client needs to determine spacial audio, attenuation,
                                        // deafening, and whether this is a group chat or not.
                                        // Each audio frame needs to be sent to a player specific, SpatialSink
    
                                        // Get the sink for the current player
                                        let source = packet.get_author();
                                        
                                        let current_player = match players.get(&current_player_name.clone()) {
                                            Some(player) => Some(player),
                                            None => None
                                        };

                                        // The current player doesn't have a position. This just means we're missing data from the server
                                        // Keep looping until we have data.
                                        if current_player.is_none() {
                                            continue;
                                        }
                                        
                                        let player_gain_settings: PlayerGainSettings = match player_gain_store_settings.0.get(&source) {
                                            Some(settings) => settings.clone(),
                                            None => PlayerGainSettings {
                                                gain: 1.0,
                                                muted: false
                                            }
                                        };

                                        // If the source is muted, ignore the packet entirely
                                        if player_gain_settings.muted {
                                            continue;
                                        }

                                        let is_spatial = match packet.spatial {
                                            Some(spatial) => spatial,
                                            None => false
                                        };

                                        match is_spatial && current_player.is_some() {
                                            true => {
                                                // We will only do positional audio if both the source and current have valid data
                                                if packet.coordinate.is_some() {
                                                    let current_player = current_player.unwrap();
                                                    // Derive the SpatialSink
                                                    let sink = sink_manager.get_sink(
                                                        source,
                                                        super::sink_manager::AudioSinkType::SpatialSink
                                                    );
                                                    let sink: Result<Arc<SpatialSink>, ()> = sink.try_into();
                                                    match sink {
                                                        Ok(sink) => {
                                                            let emitter = packet.coordinate.unwrap();
                                                            let emitter_owner = match packet.owner.clone() {
                                                                Some(owner) => match players.get(&owner.name) {
                                                                    Some(player) => Some(player),
                                                                    None => None
                                                                },
                                                                None => None
                                                            };

                                                            let deafen_emitter = match emitter_owner {
                                                                Some(owner) => owner.deafen,
                                                                None => false
                                                            };

                                                            let listener = current_player.coordinates;
                                                            let listener_orientation = current_player.orientation;
                                                            
                                                            let spatial = OutputStream::calculate_virtual_listener_audio_data(
                                                                &emitter,
                                                                deafen_emitter,
                                                                &listener,
                                                                &listener_orientation
                                                            );

                                                            sink.set_emitter_position([spatial.emitter.x, spatial.emitter.y, spatial.emitter.z]);
                                                            sink.set_left_ear_position([spatial.left_ear.x, spatial.left_ear.y, spatial.left_ear.z]);
                                                            sink.set_right_ear_position([spatial.right_ear.x, spatial.right_ear.y, spatial.right_ear.z]);

                                                            // Get player attenuation (default to 1.0 if not set) and adjust the volume of the sink
                                                            sink.set_volume(spatial.gain * player_gain_settings.gain);

                                                            sink.append(packet.buffer);
                                                        },
                                                        Err(_) => {
                                                            error!("Spatial Sink undefined");
                                                        }
                                                    };
                                                }
                                            },
                                            false => {
                                                let sink = sink_manager.get_sink(
                                                    source,
                                                    super::sink_manager::AudioSinkType::Sink
                                                );
    
                                                let sink: Result<Arc<Sink>, ()> = sink.try_into();
                                                match sink {
                                                    Ok(sink) => {
                                                        // Get player attenuation (default to 1.0 if not set) and adjust the volume of the sink
                                                        sink.set_volume(1.3 * player_gain_settings.gain);
                                                        sink.append(packet.buffer);
                                                    },
                                                    Err(_) => {}
                                                };
                                            }
                                        };                                    
                                    },
                                    Err(e) => {
                                        warn!("Could not receive decode audio frame packet: {:?}", e);
                                    }
                                }
                            }
                            log::info!("ending loop");
                        }),
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!("Couldn't retrieve native cpal device for {} {}.", device.io.to_string(), device.display_name))
                        }
                    };

                    return Ok(handle);
                },
                Err(e) => {
                    error!("Receiving stream startup failed: {:?}", e);
                    return Err(e)
                }
            },
            None => {
                return Err(anyhow!(
                    "Output Stream is not initialized with a device! Unable to start stream"
                ))
            }
        };
    }

    /// Processes AudioFramePacket data
    async fn handle_audio_data(
        decoders: &Cache<Vec<u8>, Arc<Mutex<opus::Decoder>>>,
        device_config: &rodio::cpal::StreamConfig,
        producer: Arc<Sender<DecodedAudioFramePacket>>,
        data: &QuicNetworkPacket
    ) {
        let owner = data.owner.clone();
        let client_id = data.get_client_id();
        let sample_rate = device_config.sample_rate.0.into();

        let data: Result<AudioFramePacket, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data) => {
                let decoder = match decoders.get(&client_id).await {
                    Some(decoder) => decoder.to_owned(),
                    None => {
                        let decoder = opus::Decoder
                            ::new(sample_rate, opus::Channels::Mono)
                            .unwrap();
                        let decoder = Arc::new(Mutex::new(decoder));
                        decoders.insert(client_id.clone(), decoder.clone()).await;
                        decoder
                    }
                };

                let mut decoder = decoder.lock().await;
                let mut out = vec![0.0; BUFFER_SIZE as usize];
                let out_len = match decoder.decode_float(&data.data, &mut out, false) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Could not decode audio frame packet: {:?}", e);
                        0
                    }
                };

                out.truncate(out_len);
                if out.len() > 0 {
                    let result = producer.send(DecodedAudioFramePacket {
                        owner: owner.clone(),
                        buffer: SamplesBuffer::new(
                            1,
                            data.sample_rate,
                            out
                        ),
                        coordinate: data.coordinate,
                        spatial: data.spatial,
                        orientation: data.orientation
                    });

                    match result {
                        Ok(_) => {},
                        Err(e) => warn!("{:?}", e)
                    }
                }
            },
            Err(_) => {
                warn!("Could not decode audio frame packet");
            }
        }
    }

    // Sender<AudioFrame> is technically as alias of Sender<QuicNetworkPacket> with a nested data
    // The data we can receive can be _any_ valid QuicNetworkPacket, which is good because
    // We need the positional information that is pulsed by the server
    // @todo!() we need to move this outside of this thread so this thread is only concerned with AudioFramePacket
    async fn handle_player_data(
        player_data: Arc<moka::sync::Cache<String, Player>>,
        data: &QuicNetworkPacket
    ) {
        let data: Result<PlayerDataPacket, ()> = data.data.to_owned().try_into();
        match data {
            Ok(data) => {
                for player in data.players {
                    player_data.insert(player.name.clone(), player);
                }
            },
            Err(_) => {
                warn!("Could not decode player data packet");
            }
        }
    }

    // Determines the virtual listener position based on the emitter and listener coordinates and orientation
    // And returns where their ears _should be_ in the 3D space
    fn calculate_virtual_listener_audio_data(
        emitter: &Coordinate,
        deafen_emitter: bool,
        listener: &Coordinate,
        orientation: &Orientation,
    ) -> SpatialAudioData {
        // Compute delta and full 3D distance for gain
        let dx = emitter.x - listener.x;
        let dy = emitter.y - listener.y;
        let dz = emitter.z - listener.z;

        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Constants
        let virtual_distance = 1.33;
        let close_threshold = 12.0;
        let falloff_distance = 48.0;
        let steepen_start = 38.0;
        let deafen_distance = 3.0;
        let deafen_multiplier = 0.35;

        let target_min_volume = 1.0 / (12.0 * 12.0);
        let target_max_volume = 1.0 / (virtual_distance * virtual_distance);

        // Direction vector in full 3D
        let direction = if distance > 0.01 {
            [dx / distance, dy / distance, dz / distance]
        } else {
            [0.0, 0.0, -1.0]
        };

        // Virtual listener position logic
        let virtual_listener = if distance <= close_threshold {
            Coordinate {
                x: emitter.x - direction[0] * virtual_distance,
                y: emitter.y - direction[1] * virtual_distance,
                z: emitter.z - direction[2] * virtual_distance,
            }
        } else if distance <= falloff_distance {
            let t = (distance - close_threshold) / (falloff_distance - close_threshold); // 0 → 1
            let mut volume = target_max_volume + t * (target_min_volume - target_max_volume);

            if distance >= steepen_start {
                let s = (distance - steepen_start) / (falloff_distance - steepen_start); // 0 → 1
                let steep_factor = s.powf(2.0); // steeper near end
                volume *= 1.0 - 0.5 * steep_factor; // reduce volume more aggressively
            }

            let mapped_distance = 1.0 / volume.sqrt();
            Coordinate {
                x: emitter.x - direction[0] * mapped_distance,
                y: emitter.y - direction[1] * mapped_distance,
                z: emitter.z - direction[2] * mapped_distance,
            }
        } else {
            listener.clone()
        };

        // Compute yaw (rotation about Y axis)
        let yaw_rad = orientation.y.to_radians();
        let forward_x = yaw_rad.sin();
        let forward_z = -yaw_rad.cos();
        let left_x = -forward_z;
        let left_z = forward_x;
        let ear_offset = 0.3;

        let mut left_ear = Coordinate {
            x: virtual_listener.x + left_x * ear_offset,
            y: virtual_listener.y,
            z: virtual_listener.z + left_z * ear_offset,
        };
        let mut right_ear = Coordinate {
            x: virtual_listener.x - left_x * ear_offset,
            y: virtual_listener.y,
            z: virtual_listener.z - left_z * ear_offset,
        };

        // There's stereo inversion at 24 units away???
        if distance >= 24.0 {
            right_ear = Coordinate {
                x: virtual_listener.x + left_x * ear_offset,
                y: virtual_listener.y,
                z: virtual_listener.z + left_z * ear_offset,
            };
            left_ear = Coordinate {
                x: virtual_listener.x - left_x * ear_offset,
                y: virtual_listener.y,
                z: virtual_listener.z - left_z * ear_offset,
            };
        }

        // Gain logic
        let mut gain = match deafen_emitter {
            true => {
                if distance <= deafen_distance {
                    1.0 * deafen_multiplier
                } else {
                    0.0
                }
            },
            false => {
                if distance <= falloff_distance {
                    1.0
                } else {
                    0.0
                }
            }
        };

        // If the audio output is mued, set the gain to 0 so we don't get a weird pop[]
        let is_globally_deafened = MUTE_OUTPUT_STREAM.load(Ordering::Relaxed);
        if is_globally_deafened {
            gain = 0.0;
        }
        
        //info!("Calculated spatial audio data: emitter: {:?}, source: {:?}, gain: {}, distance: {:?}", emitter, listener, gain, distance);
        SpatialAudioData {
            emitter: emitter.clone(),
            left_ear,
            right_ear,
            gain,
        }
    }

    pub fn mute(&self) {
        let current_state = MUTE_OUTPUT_STREAM.load(Ordering::Relaxed);
        MUTE_OUTPUT_STREAM.store(!current_state, Ordering::Relaxed);
    }

    pub fn mute_status(&self) -> bool {
        MUTE_OUTPUT_STREAM.load(Ordering::Relaxed)
    }
}
