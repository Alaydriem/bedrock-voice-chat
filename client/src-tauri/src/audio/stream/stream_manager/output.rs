use crate::AudioPacket;
use common::{
    structs::{
        audio::{AudioDevice, BUFFER_SIZE},
        packet::{AudioFramePacket, PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket}
    }, Coordinate, Player
};
use base64::{engine::general_purpose, Engine as _};
use log::{error, warn};
use rodio::{
    buffer::SamplesBuffer, Sink, SpatialSink
};
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
        Mutex
    }, time::Duration
};
use tokio::task::{AbortHandle, JoinHandle};
use moka::sync::Cache;
use anyhow::anyhow;

use super::sink_manager::SinkManager;

/// This is our decoded audio stream
#[derive(Debug, Clone)]
struct DecodedAudioFramePacket {
    pub owner: Option<PacketOwner>,
    pub buffer: SamplesBuffer<f32>,
    pub coordinate: Option<Coordinate>,
    pub spatial: bool
}

impl DecodedAudioFramePacket {
    pub fn get_author(&self) -> String {
        match &self.owner {
            Some(owner) => {
                // If the owner name is empty, or comes from the API, then default to the client ID
                if owner.name.eq(&"") || owner.name.eq(&"api") {
                    return general_purpose::STANDARD.encode(&owner.client_id);
                }

                return owner.name.clone();
            }
            None => String::from("")
        }
    }
}

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    players: Arc<Cache<String, Player>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    metadata: Arc<moka::sync::Cache<String, String>>
}

impl super::StreamTrait for OutputStream {
    fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        self.metadata.insert(key, value);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        // Then hard terminate them
        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        Ok(())
    }

    fn is_stopped(&mut self) -> bool {
        self.jobs.len() == 0
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);
        
        let mut jobs = vec![];
        let (producer, consumer) = mpsc::channel();

        // Playback the PCM data
        match self.playback(consumer, self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Listen to the network stream
        match self.listener(producer, self.shutdown.clone()) {
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
    pub fn new(device: Option<AudioDevice>, bus: Arc<flume::Receiver<AudioPacket>>) -> Self {
        let player_data = Cache::builder()
            .time_to_idle(Duration::from_secs(15 * 60))
            .build();

        Self {
            device,
            bus,
            players: Arc::new(player_data),
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata: Arc::new(moka::sync::Cache::builder().build())
        }
    }
    
    /// Listens to incoming network packet events from the server
    /// Translates them, then sends them to playback for processing
    fn listener(
        &mut self,
        producer: Sender<DecodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let device_config = rodio::cpal::StreamConfig {
                        channels: config.channels(),
                        sample_rate: config.sample_rate(),
                        buffer_size: cpal::BufferSize::Fixed(common::structs::audio::BUFFER_SIZE),
                    };

                    let bus = self.bus.clone();
                    let player_data = self.players.clone();

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
                            log::info!("received network packet from server for OUTPUT STREAM");
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
                                        ),
                                        PacketType::PlayerData => OutputStream::handle_player_data(
                                            player_data.clone(),
                                            &packet.data
                                        ),
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
    fn playback(
        &mut self,
        consumer: Receiver<DecodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(config) => {
                    let metadata = self.metadata.clone();
                    let players = self.players.clone();

                    let cpal_device: Option<rodio::cpal::Device> = device.clone().into();
                    let handle = match cpal_device {
                        Some(cpal_device) => tokio::spawn(async move {
                            // Only allow a sink to be active for 15 minutes
                            // This is a per _player_ sink, and ignores the client id.
                            // Client IDs can be mapped back to Strings if the author is a "device"
                            // All sinks are spatial, but some spatial sinks occur in the same space
    
                            let (_stream, handle) = match rodio::OutputStream::try_from_device_config(
                                &cpal_device,
                                config
                            ) {
                                Ok((s, h)) => (s, h),
                                Err(e) => {
                                    error!("Could not acquired Stream Handle to Output to Audio Stream. Try restarting the stream? {:?}", e);
                                    return;
                                }
                            };
                            
                            let mut sink_manager = SinkManager::new(&handle);
    
                            // Iterate over the incoming PCM data
                            #[allow(irrefutable_let_patterns)]
                            while let packet = consumer.recv() {
                                log::info!("recieved pcm stream to playback on device.");
                                if shutdown.load(Ordering::Relaxed) {
                                    warn!("{} {} ended.", device.io.to_string(), device.display_name);
                                    break;
                                }
    
                                match packet {
                                    Ok(packet) => {
                                        // Clients only hear audio that the server determins they can hear
                                        // However the client needs to determine spacial audio, attenuation,
                                        // deafening, and whether this is a group chat or not.
                                        // Each audio frame needs to be sent to a player specific, SpatialSink
    
                                        // Get the sink for the current player
                                        let source = packet.get_author();
    
                                        let current_player = match metadata.get("current_player") {
                                            Some(player) => match players.get(&player) {
                                                Some(player) => Some(player),
                                                None => None
                                            },
                                            None => None
                                        };
    
                                        if current_player.is_none() {
                                            error!("Audio stream is running without an active player. Aborting OutputStream thread");
                                            return;
                                        }
    
                                        match packet.spatial && current_player.is_some() {
                                            true => {
                                                // We will only do positional audio if both the source and current have valid data
                                                if packet.coordinate.is_some() && current_player.is_some() {
                                                    // Derive the SpatialSink
                                                    let sink = sink_manager.get_sink(
                                                        source,
                                                        super::sink_manager::AudioSinkType::SpatialSink
                                                    );
                                                    let sink: Result<Arc<SpatialSink>, ()> = sink.try_into();
                                                    match sink {
                                                        Ok(sink) => {
                                                            // Always use the packet data instead of the source data
                                                            let listener = current_player.unwrap();
            
                                                            let s = packet.coordinate.unwrap();
                                                            let l = listener.coordinates;
                                                            
                                                            // The audio should always be in the same dimensions, and within a valud coordinate range
                                                            // The client just has to attenuate the signal
                                                            sink.set_emitter_position([s.x, s.y, s.z]);
                                                            sink.set_left_ear_position([l.x + 0.0001, l.y, l.z]);
                                                            sink.set_right_ear_position([l.x, l.y, l.z]);
            
                                                            let distance = (
                                                                (s.x - l.x).powf(2.0) +
                                                                (s.y - l.y).powf(2.0) +
                                                                (s.z - l.z).powf(2.0)
                                                            ).sqrt();
            
                                                            if distance > 44.0 {
                                                                let dropoff = distance - 44.0;
                    
                                                                // This provides a 10 block linear attenuation dropoff
                                                                // y = (⁻¹⁄₁₂)x + (¹⁴⁄₃)
                                                                let dropoff_attenuation = f32::max(
                                                                    0.0,
                                                                    (-1.0 / 12.0) * dropoff + 14.0 / 3.0
                                                                );
            
                                                                sink.set_volume(dropoff_attenuation);
                                                            } else {
                                                                sink.set_volume(1.0);
                                                            }
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
                                                        // @todo: We need to pull down any player customizable attenuation
                                                        sink.set_volume(1.0);
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
                        }),
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!("Couldn't retrieve native cpal device for {} {}.", device.io.to_string(), device.display_name))
                        }
                    };

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

    /// Processes AudioFramePacket data
    fn handle_audio_data(
        decoders: &Cache<Vec<u8>, Arc<Mutex<opus::Decoder>>>,
        device_config: &rodio::cpal::StreamConfig,
        producer: Sender<DecodedAudioFramePacket>,
        data: &QuicNetworkPacket
    ) {
        let owner = data.owner.clone();
        let client_id = data.get_client_id();
        let sample_rate = device_config.sample_rate.0.into();

        let data: Result<AudioFramePacket, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data ) => {
                let decoder = match decoders.get(&client_id) {
                    Some(decoder) => decoder.to_owned(),
                    None => {
                        let decoder = opus::Decoder
                            ::new(sample_rate, opus::Channels::Mono)
                            .unwrap();
                        let decoder = Arc::new(Mutex::new(decoder));
                        decoders.insert(client_id.clone(), decoder.clone());
                        decoder
                    }
                };

                match decoder.lock() {
                    Ok(mut decoder) => {
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
                            _ = producer.send(DecodedAudioFramePacket {
                                owner: owner.clone(),
                                buffer: SamplesBuffer::<f32>::new(
                                    1, // is this _always_ a mono channel? Shouldn't this be stero sometimes too?
                                    data.sample_rate,
                                    out
                                ),
                                coordinate: data.coordinate,
                                spatial: data.spatial
                            })
                        }
                    },
                    Err(e) => {
                        warn!("Could not retrieve decoder: {:?}", e);
                    }
                };
            },
            Err(_) => {
                warn!("Couldnot decode audio frame packet");
            }
        }
    }

    // Sender<AudioFrame> is technically as alias of Sender<QuicNetworkPacket> with a nested data
    // The data we can receive can be _any_ valid QuicNetworkPacket, which is good because
    // We need the positional information that is pulsed by the server
    // @todo!() we need to move this outside of this thread so this thread is only concerned with AudioFramePacket
    fn handle_player_data(
        player_data: Arc<Cache<String, Player>>,
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
}
