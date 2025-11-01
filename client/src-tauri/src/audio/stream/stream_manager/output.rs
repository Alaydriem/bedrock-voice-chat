use super::sink_manager::SinkManager;
use crate::audio::stream::stream_manager::AudioSinkType;
use crate::audio::recording::{RawRecordingData, RecordingProducer};
use crate::{audio::stream::jitter_buffer::EncodedAudioFramePacket, events::ServerError};

use crate::audio::types::AudioDevice;
use crate::AudioPacket;
use anyhow::anyhow;
use base64::engine::{general_purpose, Engine};
use common::{
    structs::{
        audio::{PlayerGainSettings, PlayerGainStore},
        packet::{
            AudioFramePacket, ChannelEventPacket, ConnectionEventType, PacketType, PlayerDataPacket,
            PlayerPresenceEvent, QuicNetworkPacket, ServerErrorPacket,
        },
    },
    Player, PlayerData,
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

/// Global mute state for output stream
static MUTE_OUTPUT_STREAM: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub(crate) struct OutputStream {
    pub device: Option<AudioDevice>,
    pub bus: Arc<flume::Receiver<AudioPacket>>,
    players: Arc<moka::sync::Cache<String, Player>>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    pub metadata: Arc<Cache<String, String>>,
    app_handle: tauri::AppHandle,
    sink_manager: Option<SinkManager>,
    playback_stream: Option<rodi7o::OutputStream>,
    player_presence: Arc<moka::sync::Cache<String, ()>>,
    player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
    client_id_to_player: Arc<moka::sync::Cache<String, String>>,
    recording_producer: Option<Arc<RecordingProducer>>,
    player_gain_cache: Arc<moka::sync::Cache<String, PlayerGainSettings>>,
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
                        for (player_name, gain_settings) in &settings.0 {
                            self.player_gain_cache.insert(player_name.clone(), gain_settings.clone());
                        }

                        if let Some(sink_manager) = self.sink_manager.as_mut() {
                            let mut remapped_settings = PlayerGainStore::default();

                            for (player_name, gain_settings) in &settings.0 {
                                for (client_id, mapped_player_name) in
                                    self.client_id_to_player.iter()
                                {
                                    if mapped_player_name.as_str() == player_name {
                                        remapped_settings.0.insert(
                                            client_id.as_ref().clone(),
                                            gain_settings.clone(),
                                        );
                                    }
                                }
                            }

                            sink_manager.update_player_store(remapped_settings)
                        }
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

        // Note: RecordingManager is managed separately and doesn't need to be stopped here
        // The lifecycle is controlled by the main application

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
        recording_producer: Option<Arc<RecordingProducer>>,
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

        let client_id_to_player = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(3 * 60))
            .build();

        let player_gain_cache = moka::sync::Cache::builder()
            .time_to_idle(Duration::from_secs(3 * 60))
            .build();

        Self {
            device,
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
            client_id_to_player: Arc::new(client_id_to_player),
            recording_producer,
            player_gain_cache: Arc::new(player_gain_cache),
        }
    }

    /// Listens to incoming network packet events from the server
    /// Translates them, then sends them to playback for processing
    async fn listener(
        &mut self,
        producer: flume::Sender<EncodedAudioFramePacket>,
        shutdown: Arc<AtomicBool>,
        players: Arc<moka::sync::Cache<String, Player>>,
        metadata: Arc<Cache<String, String>>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        match self.device.clone() {
            Some(device) => match device.get_stream_config() {
                Ok(_config) => {
                    let bus = self.bus.clone();

                    let player_presence = self.player_presence.clone();
                    let player_presence_debounce = self.player_presence_debounce.clone();
                    let client_id_to_player = self.client_id_to_player.clone();
                    let app_handle = self.app_handle.clone();
                    let recording_producer = self.recording_producer.clone();

                    let player_gain_cache = self.player_gain_cache.clone();

                    let handle = tokio::spawn(async move {
                        #[allow(irrefutable_let_patterns)]
                        while let packet = bus.recv_async().await {
                            if shutdown.load(Ordering::Relaxed) {
                                break;
                            }

                            match packet {
                                Ok(packet) => {

                                    match packet.data.get_packet_type() {
                                        PacketType::AudioFrame => {
                                            OutputStream::handle_audio_data(
                                                producer.clone(),
                                                &packet.data,
                                                metadata.clone(),
                                                players.clone(),
                                                player_gain_cache.clone(),
                                                player_presence.clone(),
                                                player_presence_debounce.clone(),
                                                client_id_to_player.clone(),
                                                Some(&app_handle.clone()),
                                                recording_producer.as_ref().map(|p| (**p).clone()),
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
                                        PacketType::ServerError => {
                                            OutputStream::handle_server_error(
                                                &packet.data,
                                                Some(&app_handle.clone()),
                                            )
                                            .await
                                        }
                                        PacketType::PlayerPresence => {
                                            OutputStream::handle_player_presence(
                                                &packet.data,
                                                metadata.clone(),
                                                Some(&app_handle.clone()),
                                                player_presence.clone(),
                                                player_presence_debounce.clone(),
                                            )
                                            .await
                                        }
                                        PacketType::ChannelEvent => {
                                            OutputStream::handle_channel_event(
                                                &packet.data,
                                                Some(&app_handle.clone()),
                                            )
                                            .await
                                        }
                                        _ => {}
                                    }
                                },
                                Err(_e) => {
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
                                self.app_handle.clone(),
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

    // Process the player presence event
    async fn handle_player_presence(
        data: &QuicNetworkPacket,
        metadata: Arc<Cache<String, String>>,
        app_handle: Option<&tauri::AppHandle>,
        player_presence: Arc<moka::sync::Cache<String, ()>>,
        player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
    ) {
        let current_player_name = match metadata.get("current_player").await {
            Some(name) => name,
            None => return,
        };

        if let Some(app_handle) = app_handle {
            let data: Result<PlayerPresenceEvent, ()> = data.data.to_owned().try_into();

            match data {
                Ok(data) => {
                    // Ignore events from self
                    if current_player_name.eq(&data.player_name) {
                        return;
                    }

                    match data.event_type {
                        ConnectionEventType::Connected => {
                            player_presence.insert(data.player_name.clone(), ());

                            // Only emit if not recently debounced
                            if player_presence_debounce.get(&data.player_name).is_none() {
                                player_presence_debounce.insert(data.player_name.clone(), ());

                                if let Err(e) = app_handle.emit(
                                    crate::events::event::player_presence::PLAYER_PRESENCE,
                                    crate::events::event::player_presence::Presence::new(
                                        data.player_name.clone(),
                                        String::from("joined"),
                                    ),
                                ) {
                                    error!("Failed to emit player presence event: {:?}", e);
                                }
                            }
                        }
                        ConnectionEventType::Disconnected => {
                            player_presence.remove(&data.player_name);
                            player_presence_debounce.remove(&data.player_name);

                            if let Err(e) = app_handle.emit(
                                crate::events::event::player_presence::PLAYER_PRESENCE,
                                crate::events::event::player_presence::Presence::new(
                                    data.player_name.clone(),
                                    String::from("disconnected"),
                                ),
                            ) {
                                error!("Failed to emit player presence event: {:?}", e);
                            }
                        }
                    }
                }
                Err(_) => {
                    warn!("Could not decode player data packet");
                }
            }
        }
    }

    // Process channel events (create, delete, join, leave)
    async fn handle_channel_event(
        data: &QuicNetworkPacket,
        app_handle: Option<&tauri::AppHandle>,
    ) {
        if let Some(app_handle) = app_handle {
            let channel_event: Result<ChannelEventPacket, ()> = data.data.to_owned().try_into();

            match channel_event {
                Ok(event) => {
                    let event_type = match event.event {
                        common::structs::channel::ChannelEvents::Create => "create",
                        common::structs::channel::ChannelEvents::Delete => "delete",
                        common::structs::channel::ChannelEvents::Join => "join",
                        common::structs::channel::ChannelEvents::Leave => "leave",
                    };

                    info!(
                        "Channel event: {} {} in channel {} ({})",
                        event.name,
                        event_type,
                        event.channel,
                        event.channel_name.as_deref().unwrap_or("unknown")
                    );

                    if let Err(e) = app_handle.emit(
                        crate::events::event::channel_event::CHANNEL_EVENT,
                        crate::events::event::channel_event::ChannelEvent::new(
                            event_type.to_string(),
                            event.channel,
                            event.channel_name,
                            event.creator,
                            event.name,
                            event.timestamp,
                        ),
                    ) {
                        error!("Failed to emit channel event: {:?}", e);
                    }
                }
                Err(_) => {
                    warn!("Could not decode channel event packet");
                }
            }
        }
    }

    /// Processes AudioFramePacket data
    async fn handle_audio_data(
        producer: flume::Sender<EncodedAudioFramePacket>,
        data: &QuicNetworkPacket,
        metadata: Arc<Cache<String, String>>,
        players: Arc<moka::sync::Cache<String, Player>>,
        player_gain_cache: Arc<moka::sync::Cache<String, PlayerGainSettings>>,
        player_presence: Arc<moka::sync::Cache<String, ()>>,
        player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
        client_id_to_player: Arc<moka::sync::Cache<String, String>>,
        app_handle: Option<&tauri::AppHandle>,
        recording_producer: Option<crate::audio::recording::RecordingProducer>,
    ) {
        let current_player_name = match metadata.get("current_player").await {
            Some(name) => name,
            None => return,
        };

        // Check if this is a new player we haven't seen before
        if let Some(owner) = &data.owner {
            let player_name = &owner.name;

            // Build client ID to player name mapping for gain control
            if !player_name.is_empty() && !player_name.eq(&"api") {
                let client_id = general_purpose::STANDARD.encode(&owner.client_id);
                client_id_to_player.insert(client_id, player_name.clone());
            }

            // Don't emit events for ourselves
            if !player_name.eq(&current_player_name) && !player_name.is_empty() {
                // Always update the presence cache
                player_presence.insert(player_name.clone(), ());

                // Only emit if not recently debounced
                if player_presence_debounce.get(player_name).is_none() {
                    player_presence_debounce.insert(player_name.clone(), ());

                    // Emit synthetic presence event for new player detected via audio
                    if let Some(app_handle) = app_handle {
                        if let Err(e) = app_handle.emit(
                            crate::events::event::player_presence::PLAYER_PRESENCE,
                            crate::events::event::player_presence::Presence::new(
                                player_name.clone(),
                                String::from("joined"),
                            ),
                        ) {
                            error!(
                                "Failed to emit auto-detected player presence event: {:?}",
                                e
                            );
                        }
                    }
                }
            }
        }

        let owner = data.owner.clone();
        let data: Result<AudioFramePacket, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data) => {
                // Create emitter PlayerData from packet owner and audio data
                let emitter = owner
                    .as_ref()
                    .map(|o| PlayerData::from_packet_owner(
                        o,
                        &data,
                        player_gain_cache.get(&o.name),
                    ))
                    .unwrap_or_else(PlayerData::unknown);

                // Create listener PlayerData from current player
                let listener = players
                    .get(&current_player_name)
                    .map(|p| PlayerData::from_player(
                        &p,
                        current_player_name.clone(),
                        player_gain_cache.get(&current_player_name),
                    ))
                    .unwrap_or_else(|| PlayerData::unknown());

                let encoded_packet = EncodedAudioFramePacket {
                    timestamp: data.timestamp() as u64,
                    sample_rate: data.sample_rate,
                    data: data.data,
                    route: AudioSinkType::from_spatial(match data.spatial {
                        Some(s) => s,
                        None => false,
                    }),
                    emitter,
                    listener,
                    buffer_size_ms: 120,
                    time_between_reports_secs: 30,
                };

                // Send to playback
                match producer.send(encoded_packet.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Could not send encoded audio frame packet: {:?}", e);
                    }
                }

                // Send to recording if producer available
                if let Some(ref producer) = recording_producer {
                    // Use default channel count since detect_opus_channels was removed
                    let recording_data = RawRecordingData::OutputData {
                        absolute_timestamp_ms: Some(std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64),
                        opus_data: encoded_packet.data.clone(),
                        sample_rate: encoded_packet.sample_rate,
                        channels: 1, // Default to mono
                        emitter: encoded_packet.emitter.clone(),
                        listener: encoded_packet.listener.clone(),
                        is_spatial: encoded_packet.emitter.spatial.unwrap_or(false),
                    };
                    let _ = producer.try_send(recording_data);
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

    async fn handle_server_error(data: &QuicNetworkPacket, app_handle: Option<&tauri::AppHandle>) {
        if let Some(app_handle) = app_handle {
            if let Ok(error_packet) = TryInto::<ServerErrorPacket>::try_into(data.data.clone()) {
                if let Err(e) = app_handle.emit(
                    crate::events::event::server_error::SERVER_ERROR,
                    ServerError::new(error_packet.error_type, error_packet.message),
                ) {
                    error!("Failed to emit server error event: {:?}", e);
                }
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
