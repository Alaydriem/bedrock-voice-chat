use crate::audio::stream::jitter_buffer::EncodedAudioFramePacket;
use crate::audio::stream::stream_manager::AudioSinkType;

use crate::audio::types::AudioDevice;
use crate::AudioPacket;
use anyhow::anyhow;
use common::{
    structs::{
        audio::PlayerGainStore,
        packet::{AudioFramePacket, PacketType, PlayerDataPacket, QuicNetworkPacket},
    },
    Player,
};
use log::{error, info, warn};
use moka::future::Cache;
use once_cell::sync::Lazy;
use rodio::OutputStreamBuilder;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::Duration,
};
use tauri::Emitter;
use tokio::task::{AbortHandle, JoinHandle};

use super::sink_manager::SinkManager;

static MUTE_OUTPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    players: Arc<moka::sync::Cache<String, Player>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<Cache<String, String>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    player_presence: Arc<moka::sync::Cache<String, bool>>,
    sink_manager: Option<SinkManager>,
    // Keep the rodio output stream alive for the lifetime of playback
    playback_stream: Option<rodio::OutputStream>,
}

impl common::traits::StreamTrait for OutputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        match key.as_str() {
            "mute" => {
                self.mute();
            }
            "player_gain_store" => {
                match serde_json::from_str::<PlayerGainStore>(&value) {
                    Ok(settings) => {
                        if let Some(sink_manager) = self.sink_manager.as_mut() {
                            sink_manager.update_player_store(settings.clone())
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse PlayerGainStore: {:?}", e);
                    }
                };
                info!("Player gain store updated.");
            }
            "player_presence" => {
                if !self.player_presence.contains_key(&value) {
                    self.app_handle
                        .emit(
                            crate::events::event::player_presence::PLAYER_PRESENCE,
                            crate::events::event::player_presence::Presence::new(
                                value.clone(),
                                "online".to_string(),
                            ),
                        )
                        .unwrap();
                }

                self.player_presence.insert(value.clone(), true);
            }
            _ => {
                let _ = self.metadata.insert(key.clone(), value.clone()).await;
            }
        };

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(true, Ordering::Relaxed);

        if let Some(sink_manager) = self.sink_manager.as_mut() {
            sink_manager.stop().await;
        }
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
        self.jobs.is_empty()
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let mut jobs = vec![];
        let (producer, consumer) = flume::unbounded();

        // Playback the PCM data
        match self
            .playback(
                consumer,
                self.shutdown.clone(),
                self.metadata.clone(),
                self.players.clone(),
            )
            .await
        {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("input sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Listen to the network stream
        match self
            .listener(producer, self.shutdown.clone(), self.players.clone())
            .await
        {
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
        app_handle: tauri::AppHandle,
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
            player_presence: Arc::new(
                moka::sync::Cache::builder()
                    .time_to_idle(Duration::from_secs(3 * 60))
                    .build(),
            ),
            sink_manager: None,
            playback_stream: None,
        }
    }

    /// Listens to incoming network packet events from the server
    /// Translates them, then sends them to playback for processing
    async fn listener(
        &mut self,
        producer: flume::Sender<EncodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>,
        players: Arc<moka::sync::Cache<String, Player>>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(_config) => {
                    let bus = self.bus.clone();

                    let handle = tokio::spawn(async move {
                        #[allow(irrefutable_let_patterns)]
                        while let packet = bus.recv_async().await {
                            if shutdown.load(Ordering::Relaxed) {
                                warn!("Output listener handler stopped.");
                                break;
                            }

                            match packet {
                                Ok(packet) => match packet.data.get_packet_type() {
                                    PacketType::AudioFrame => {
                                        OutputStream::handle_audio_data(
                                            producer.clone(),
                                            &packet.data,
                                        )
                                        .await
                                    }
                                    PacketType::PlayerData => {
                                        OutputStream::handle_player_data(
                                            players.clone(),
                                            &packet.data,
                                        )
                                        .await
                                    }
                                    _ => {}
                                },
                                Err(e) => {
                                    warn!("Failed to receive packet: {:?}", e);
                                }
                            }
                        }
                    });

                    return Ok(handle);
                }
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
        consumer: flume::Receiver<EncodedAudioFramePacket>,
        _shutdown: Arc<AtomicBool>,
        metadata: Arc<Cache<String, String>>,
        players: Arc<moka::sync::Cache<String, Player>>,
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
                        Some(cpal_device) => {
                            log::info!("started receiving audio stream");
                            let builder = match OutputStreamBuilder::from_device(cpal_device) {
                                Ok(b) => b,
                                Err(e) => {
                                    error!("Could not create OutputStreamBuilder: {:?}", e);
                                    return Err(anyhow::anyhow!(e));
                                }
                            };
                            let stream_config: rodio::cpal::StreamConfig = config.clone().into();
                            let builder = builder.with_config(&stream_config);
                            let stream = match builder.open_stream_or_fallback() {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("Could not acquire OutputStream. Try restarting the stream? {:?}", e);
                                    return Err(anyhow::anyhow!(e));
                                }
                            };

                            // Keep the stream alive on self first, then get mixer
                            self.playback_stream = Some(stream);
                            let mixer = self.playback_stream.as_ref().unwrap().mixer();
                            let sink_manager = SinkManager::new(
                                consumer,
                                (*players).clone(),
                                current_player_name,
                                Arc::new(StdMutex::new(PlayerGainStore::default())),
                                Arc::new(mixer.clone()),
                            );

                            self.sink_manager = Some(sink_manager);

                            // Start the sink manager
                            let listen_handle =
                                match self.sink_manager.as_mut().unwrap().listen().await {
                                    Ok(handle) => handle,
                                    Err(e) => return Err(e),
                                };

                            listen_handle
                        }
                        None => {
                            error!("CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}", device.clone());
                            return Err(anyhow::anyhow!(
                                "Couldn't retrieve native cpal device for {} {}.",
                                device.io.to_string(),
                                device.display_name
                            ));
                        }
                    };

                    return Ok(handle);
                }
                Err(e) => {
                    error!("Receiving stream startup failed: {:?}", e);
                    return Err(e);
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
        producer: flume::Sender<EncodedAudioFramePacket>,
        data: &QuicNetworkPacket,
    ) {
        let owner = data.owner.clone();
        let data: Result<AudioFramePacket, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data) => {
                // Use the packet's own sample rate, not the output device's
                let packet_rate: u32 = data.sample_rate as u32;
                let result = producer.send(EncodedAudioFramePacket {
                    timestamp: data.timestamp() as u64,
                    sample_rate: packet_rate,
                    data: data.data,
                    route: AudioSinkType::from_spatial(match data.spatial {
                        Some(s) => s,
                        None => false,
                    }),
                    coordinate: data.coordinate,
                    orientation: data.orientation,
                    dimension: data.dimension,
                    spatial: data.spatial,
                    owner: owner.clone(),
                });

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Could not send encoded audio frame packet: {:?}", e);
                    }
                }
            }
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
        data: &QuicNetworkPacket,
    ) {
        let data: Result<PlayerDataPacket, ()> = data.data.to_owned().try_into();
        match data {
            Ok(data) => {
                for player in data.players {
                    player_data.insert(player.name.clone(), player);
                }
            }
            Err(_) => {
                warn!("Could not decode player data packet");
            }
        }
    }

    pub fn mute(&self) {
        let current_state = MUTE_OUTPUT_STREAM.load(Ordering::Relaxed);
        MUTE_OUTPUT_STREAM.store(!current_state, Ordering::Relaxed);
        if let Some(sink_manager) = self.sink_manager.as_ref() {
            sink_manager.update_global_mute(!current_state);
        }
    }

    pub fn mute_status(&self) -> bool {
        MUTE_OUTPUT_STREAM.load(Ordering::Relaxed)
    }
}
