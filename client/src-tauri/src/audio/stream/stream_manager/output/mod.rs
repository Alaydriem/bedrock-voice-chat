mod router;

use super::sink_manager::SinkManager;
use crate::audio::recording::RecordingProducer;
use crate::audio::stream::RecoverySender;
use crate::audio::stream::jitter_buffer::EncodedAudioFramePacket;
use crate::audio::stream::stream_manager::AudioOutputSink;
use router::PacketRouter;

use crate::AudioPacket;
use crate::audio::types::AudioDevice;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::AnnounceInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxBeaconCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxEjectInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::PresenceInjector;
use anyhow::anyhow;
use common::{
    Coordinate, Game, GenericPlayer, Orientation, PlayerEnum,
    structs::{
        SpatialAudioConfig,
        audio::{GainProjection, PlayerGainSettings, PlayerGainStore, StreamEvent},
    },
};
use log::{error, info, warn};
use moka::future::Cache;
use once_cell::sync::Lazy;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::task::{AbortHandle, JoinHandle};

/// Global mute state for output stream
pub(crate) static MUTE_OUTPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    sink: AudioOutputSink,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    players: Arc<moka::sync::Cache<String, PlayerEnum>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<Cache<String, String>>,
    app_handle: tauri::AppHandle,
    sink_manager: Option<SinkManager>,
    playback_stream: Option<rodio::MixerDeviceSink>,
    player_presence: Arc<moka::sync::Cache<String, Option<String>>>,
    player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
    // Per-device gain and mute. Owned here because both halves that feed it live here: the
    // router observes which player each device belongs to, and the persisted store arrives as
    // a metadata write.
    gain: Arc<GainProjection>,
    recording_producer: Option<Arc<RecordingProducer>>,
    player_gain_cache: Arc<moka::sync::Cache<String, PlayerGainSettings>>,
    peer_registry: Arc<crate::diagnostics::PeerRegistry>,
    session_config: Arc<crate::diagnostics::SessionConfig>,
    recording_active: Option<Arc<AtomicBool>>,
    #[allow(unused)]
    recovery_tx: RecoverySender,
    #[cfg(feature = "bedrock-protocol")]
    beacon_cache: Option<Arc<JukeboxBeaconCache>>,
    #[cfg(feature = "bedrock-protocol")]
    eject_injector: Option<Arc<JukeboxEjectInjector>>,
    #[cfg(feature = "bedrock-protocol")]
    presence_injector: Option<Arc<PresenceInjector>>,
    #[cfg(feature = "bedrock-protocol")]
    announce_injector: Option<Arc<AnnounceInjector>>,
}

impl common::traits::StreamTrait for OutputStream {
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        match key.as_str() {
            "mute" => {
                self.toggle(StreamEvent::Mute);
            }
            "record" => {
                self.toggle(StreamEvent::Record);
            }
            "panning_intensity" => {
                if let Ok(intensity) = value.parse::<f32>() {
                    if let Some(sink_manager) = self.sink_manager.as_ref() {
                        sink_manager.update_panning_intensity(intensity);
                    }
                }
                let _ = self.metadata.insert(key.clone(), value.clone()).await;
            }
            "player_gain_store" => {
                match serde_json::from_str::<PlayerGainStore>(&value) {
                    Ok(settings) => {
                        for (player_name, gain_settings) in &settings.0 {
                            self.player_gain_cache
                                .insert(player_name.clone(), gain_settings.clone());
                        }

                        // Handed over whole. The projection resolves a device against it at
                        // lookup, so a device first heard from after this write still picks up
                        // its settings — which is what the remap this replaced could not do.
                        self.gain.set_store(settings);
                    }
                    Err(e) => {
                        error!("Failed to parse PlayerGainStore: {:?}", e);
                    }
                };
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

    #[tracing::instrument(skip(self))]
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
                error!("output sender encountered an error: {:?}", e);
                return Err(e);
            }
        };

        // Listen to the network stream
        match self
            .listener(
                producer,
                self.shutdown.clone(),
                self.players.clone(),
                self.metadata.clone(),
            )
            .await
        {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("output sender encountered an error: {:?}", e);
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
        sink: AudioOutputSink,
        bus: Arc<flume::Receiver<AudioPacket>>,
        metadata: Arc<moka::future::Cache<String, String>>,
        app_handle: tauri::AppHandle,
        recording_producer: Option<Arc<RecordingProducer>>,
        recording_active: Option<Arc<AtomicBool>>,
        recovery_tx: RecoverySender,
        peer_registry: Arc<crate::diagnostics::PeerRegistry>,
        session_config: Arc<crate::diagnostics::SessionConfig>,
        #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<Arc<JukeboxBeaconCache>>,
        #[cfg(feature = "bedrock-protocol")] eject_injector: Option<Arc<JukeboxEjectInjector>>,
        #[cfg(feature = "bedrock-protocol")] presence_injector: Option<Arc<PresenceInjector>>,
        #[cfg(feature = "bedrock-protocol")] announce_injector: Option<Arc<AnnounceInjector>>,
    ) -> Self {
        let players = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(15 * 60))
            .build();

        let player_presence = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(3 * 60))
            .build();

        let player_presence_debounce = moka::sync::Cache::builder()
            .time_to_live(Duration::from_secs(3))
            .build();

        let player_gain_cache = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(3 * 60))
            .build();

        Self {
            device,
            sink,
            bus,
            players: Arc::new(players),
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            metadata,
            app_handle: app_handle.clone(),
            sink_manager: None,
            playback_stream: None,
            player_presence: Arc::new(player_presence),
            player_presence_debounce: Arc::new(player_presence_debounce),
            gain: Arc::new(GainProjection::new()),
            recording_producer,
            player_gain_cache: Arc::new(player_gain_cache),
            peer_registry,
            session_config,
            recording_active,
            recovery_tx,
            #[cfg(feature = "bedrock-protocol")]
            beacon_cache,
            #[cfg(feature = "bedrock-protocol")]
            eject_injector,
            #[cfg(feature = "bedrock-protocol")]
            presence_injector,
            #[cfg(feature = "bedrock-protocol")]
            announce_injector,
        }
    }

    /// Listens to incoming network packet events from the server
    /// Translates them, then sends them to playback for processing
    async fn listener(
        &mut self,
        producer: flume::Sender<EncodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>,
        players: Arc<moka::sync::Cache<String, PlayerEnum>>,
        metadata: Arc<Cache<String, String>>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        // When a real device is configured, validate that it has a usable stream config.
        // When device=None (Fake backend), skip that check and fall through to spawn the listener.
        if let Some(device) = self.device.clone() {
            device.get_stream_config()?;
        }

        let bus = self.bus.clone();

        let router = PacketRouter::new(
            producer,
            metadata,
            players,
            self.player_gain_cache.clone(),
            self.player_presence.clone(),
            self.player_presence_debounce.clone(),
            self.gain.clone(),
            self.app_handle.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.beacon_cache.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.eject_injector.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.presence_injector.clone(),
            #[cfg(feature = "bedrock-protocol")]
            self.announce_injector.clone(),
        );

        let handle = tokio::spawn(async move {
            while let Ok(packet) = bus.recv_async().await {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                router.dispatch(packet).await;
            }
        });

        Ok(handle)
    }

    /// Handles playback of the PCM Audio Stream to the output device
    async fn playback(
        &mut self,
        consumer: flume::Receiver<EncodedAudioFramePacket>,
        _shutdown: Arc<AtomicBool>,
        metadata: Arc<Cache<String, String>>,
        players: Arc<moka::sync::Cache<String, PlayerEnum>>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        let current_player_name = match metadata.get("current_player").await {
            Some(name) => {
                log::info!("Starting playback for current player: '{}'", name);
                name
            }
            None => {
                return Err(anyhow!(
                    "Playback stream cannot start without a player name set. Hint: .metadata('current_player', String) first."
                ));
            }
        };

        // Seed the player cache with a default entry for the current player.
        // In group channels (non-proximity), the server may never send PlayerDataPacket,
        // so the listener would otherwise never appear in the cache.
        // This default is overwritten when real position data arrives.
        if players.get(&current_player_name).is_none() {
            players.insert(
                current_player_name.clone(),
                PlayerEnum::Generic(GenericPlayer {
                    name: current_player_name.clone(),
                    coordinates: Coordinate::default(),
                    orientation: Orientation { x: 0.0, y: 0.0 },
                    game: Game::Minecraft,
                }),
            );
        }

        // Resolve the live device's stream config for the cpal path; the fake
        // sink ignores it and builds its own in-memory mixer. The sink then
        // yields a uniform MixTarget the SinkManager feeds, so playback has one
        // body with no per-variant branching.
        let resolved_config = match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(stored_config) => {
                    // Validate stored config against live device - detect Windows sound settings changes
                    let config = match crate::audio::device::refresh_device_config(&device) {
                        Some(fresh_configs) if !fresh_configs.is_empty() => {
                            let fresh_config: rodio::cpal::SupportedStreamConfig =
                                fresh_configs[0].clone().into();
                            if fresh_config.sample_rate() != stored_config.sample_rate() {
                                warn!(
                                    "Output device {} sample rate changed: stored {}Hz, actual {}Hz. Using actual.",
                                    device.display_name,
                                    stored_config.sample_rate(),
                                    fresh_config.sample_rate()
                                );
                            }
                            fresh_config
                        }
                        _ => {
                            warn!(
                                "Could not refresh output device config for {}, using stored config",
                                device.display_name
                            );
                            stored_config
                        }
                    };
                    Some(config)
                }
                Err(e) => {
                    error!("Receiving stream startup failed: {:?}", e);
                    return Err(e);
                }
            },
            None => None,
        };

        let sink = std::mem::replace(&mut self.sink, AudioOutputSink::Rodio);
        let mix_target = sink.open(self.device.clone(), resolved_config, self.shutdown.clone())?;
        self.playback_stream = mix_target.playback_stream;

        let spatial_config = match metadata.get("spatial_audio_config").await {
            Some(json) => serde_json::from_str::<SpatialAudioConfig>(&json).unwrap_or_default(),
            None => SpatialAudioConfig::default(),
        };

        // Recorded where the value is resolved, so a diagnostic reports the range this session
        // actually runs under rather than the compiled default.
        self.session_config
            .set_spatial(spatial_config.falloff_distance, "inverse-square");

        let panning_intensity = match metadata.get("panning_intensity").await {
            Some(val) => val.parse::<f32>().unwrap_or(0.8),
            None => 0.8,
        };

        let sink_manager = SinkManager::new(
            consumer,
            (*players).clone(),
            current_player_name,
            self.gain.clone(),
            mix_target.mixer,
            self.app_handle.clone(),
            self.recording_producer.as_ref().map(|p| (**p).clone()),
            self.recording_active.clone(),
            spatial_config,
            panning_intensity,
            self.peer_registry.clone(),
        );

        self.sink_manager = Some(sink_manager);

        let listen_handle = match self.sink_manager.as_mut().unwrap().listen().await {
            Ok(handle) => handle,
            Err(e) => return Err(e),
        };

        Ok(listen_handle)
    }

    pub fn toggle(&self, event: StreamEvent) {
        match event {
            StreamEvent::Mute => {
                let current_state = MUTE_OUTPUT_STREAM.load(Ordering::Relaxed);
                MUTE_OUTPUT_STREAM.store(!current_state, Ordering::Relaxed);
                if let Some(sink_manager) = self.sink_manager.as_ref() {
                    sink_manager.update_global_mute(!current_state);
                }
            }
            StreamEvent::Record => {
                // Recording state is now owned by RecordingManager
                // Streams read the shared flag directly - no toggle needed
            }
        }
    }

    pub fn mute_status(&self) -> bool {
        MUTE_OUTPUT_STREAM.load(Ordering::Relaxed)
    }

    /// Returns the currently tracked players with their game type from the presence cache
    pub fn get_current_players(&self) -> std::collections::HashMap<String, Option<String>> {
        self.player_presence
            .iter()
            .map(|(name, game)| ((*name).clone(), Option::clone(&game)))
            .collect()
    }
}
