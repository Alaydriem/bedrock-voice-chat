use crate::AudioPacket;
use crate::audio::stream::jitter_buffer::EncodedAudioFramePacket;
use crate::audio::stream::stream_manager::AudioSinkType;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::AnnounceInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxBeaconCache;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::JukeboxEjectInjector;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::PresenceInjector;
#[cfg(feature = "bedrock-protocol")]
use common::structs::packet::AudioFrameMetadata;
#[cfg(feature = "bedrock-protocol")]
use common::structs::packet::BedrockEventPacket;
#[cfg(feature = "bedrock-protocol")]
use common::structs::packet::PeerAnnounceInjectPacket;
#[cfg(feature = "bedrock-protocol")]
use common::structs::packet::PeerPresenceInjectPacket;
#[cfg(feature = "bedrock-protocol")]
use common::structs::packet::QuicNetworkPacketData;
use common::traits::player_data::PlayerData;
use common::{
    PlayerEnum, RecordingPlayerData,
    structs::{
        audio::{GainProjection, PlayerGainSettings},
        network::ConnectionHealth,
        packet::{
            AudioFramePacket, ChannelEventPacket, ConnectionEventType, PacketType,
            ChatMessagePacket, PlayerDataPacket, PlayerPresenceEvent, QuicNetworkPacket,
            ServerErrorPacket,
            ServerErrorType,
        },
    },
};
use common::structs::analytics::{AnalyticsEvent, AnalyticsEventData};
use log::{error, info, warn};
use moka::future::Cache;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Activation is reported at most once per process, which is the same scope as the
// `$session_id` the funnel is grouped by. A router is rebuilt on every reconnect, so a
// flag on the struct would report the same activation again each time the connection
// dropped and came back.
static ACTIVATION_REPORTED: AtomicBool = AtomicBool::new(false);

pub(crate) struct PacketRouter {
    producer: flume::Sender<EncodedAudioFramePacket>,
    metadata: Arc<Cache<String, String>>,
    players: Arc<moka::sync::Cache<String, PlayerEnum>>,
    player_gain_cache: Arc<moka::sync::Cache<String, PlayerGainSettings>>,
    player_presence: Arc<moka::sync::Cache<String, Option<String>>>,
    player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
    gain: Arc<GainProjection>,
    app_handle: tauri::AppHandle,
    #[cfg(feature = "bedrock-protocol")]
    beacon_cache: Option<Arc<JukeboxBeaconCache>>,
    #[cfg(feature = "bedrock-protocol")]
    eject_injector: Option<Arc<JukeboxEjectInjector>>,
    #[cfg(feature = "bedrock-protocol")]
    presence_injector: Option<Arc<PresenceInjector>>,
    #[cfg(feature = "bedrock-protocol")]
    announce_injector: Option<Arc<AnnounceInjector>>,
}

impl PacketRouter {
    pub fn new(
        producer: flume::Sender<EncodedAudioFramePacket>,
        metadata: Arc<Cache<String, String>>,
        players: Arc<moka::sync::Cache<String, PlayerEnum>>,
        player_gain_cache: Arc<moka::sync::Cache<String, PlayerGainSettings>>,
        player_presence: Arc<moka::sync::Cache<String, Option<String>>>,
        player_presence_debounce: Arc<moka::sync::Cache<String, ()>>,
        gain: Arc<GainProjection>,
        app_handle: tauri::AppHandle,
        #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<Arc<JukeboxBeaconCache>>,
        #[cfg(feature = "bedrock-protocol")] eject_injector: Option<Arc<JukeboxEjectInjector>>,
        #[cfg(feature = "bedrock-protocol")] presence_injector: Option<Arc<PresenceInjector>>,
        #[cfg(feature = "bedrock-protocol")] announce_injector: Option<Arc<AnnounceInjector>>,
    ) -> Self {
        Self {
            producer,
            metadata,
            players,
            player_gain_cache,
            player_presence,
            player_presence_debounce,
            gain,
            app_handle,
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

    pub async fn dispatch(&self, packet: AudioPacket) {
        match packet.data.get_packet_type() {
            PacketType::AudioFrame => self.handle_audio_data(&packet.data).await,
            PacketType::PlayerData => self.handle_player_data(&packet.data).await,
            PacketType::ServerError => self.handle_server_error(&packet.data).await,
            PacketType::PlayerPresence => self.handle_player_presence(&packet.data).await,
            PacketType::ChannelEvent => self.handle_channel_event(&packet.data).await,
            PacketType::ChatMessage => self.handle_chat_message(&packet.data).await,
            #[cfg(feature = "bedrock-protocol")]
            PacketType::BedrockEvent => {
                if let Some(injector) = self.eject_injector.as_ref() {
                    if let Some(data) = packet.data.get_data() {
                        let decoded: Result<BedrockEventPacket, ()> = data.to_owned().try_into();
                        if let Ok(event_packet) = decoded {
                            injector.handle_packet(&event_packet);
                        }
                    }
                }
            }
            #[cfg(feature = "bedrock-protocol")]
            PacketType::PeerPresenceInject => {
                if let Some(injector) = self.presence_injector.as_ref() {
                    if let Some(QuicNetworkPacketData::PeerPresenceInject(inject)) =
                        packet.data.get_data()
                    {
                        let inject: &PeerPresenceInjectPacket = inject;
                        injector.handle_inject(inject);
                    }
                }
            }
            #[cfg(feature = "bedrock-protocol")]
            PacketType::PeerAnnounceInject => {
                if let Some(injector) = self.announce_injector.as_ref() {
                    if let Some(QuicNetworkPacketData::PeerAnnounceInject(inject)) =
                        packet.data.get_data()
                    {
                        let inject: &PeerAnnounceInjectPacket = inject;
                        injector.handle_inject(inject);
                    }
                }
            }
            PacketType::ClientAction => {
                if let Some(common::structs::packet::QuicNetworkPacketData::ClientAction(p)) =
                    packet.data.get_data()
                {
                    if p.direction == common::structs::packet::PacketDirection::ClientBound {
                        // Hand the action to the app-level consumer over the
                        // control channel; try_state so contexts wired without
                        // build_managed_state simply drop it.
                        if let Some(tx) = tauri::Manager::try_state::<
                            crate::control::ControlActionSender,
                        >(&self.app_handle)
                        {
                            tx.send(p.action.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Process the player presence event
    async fn handle_player_presence(&self, data: &QuicNetworkPacket) {
        let current_player_name = match self.metadata.get("current_player").await {
            Some(name) => name,
            None => return,
        };

        let data: Result<PlayerPresenceEvent, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data) => {
                // Ignore events from self
                if current_player_name.eq(&data.player_name) {
                    return;
                }

                let game = self
                    .players
                    .get(&data.player_name)
                    .map(|p| p.get_game().as_str().to_string())
                    .or_else(|| self.player_presence.get(&data.player_name).flatten());

                match data.event_type {
                    ConnectionEventType::Connected => {
                        self.player_presence
                            .insert(data.player_name.clone(), game.clone());

                        // Only emit if not recently debounced
                        if self
                            .player_presence_debounce
                            .get(&data.player_name)
                            .is_none()
                        {
                            self.player_presence_debounce
                                .insert(data.player_name.clone(), ());

                            if let Err(e) = tauri::Emitter::emit(
                                &self.app_handle,
                                crate::events::event::player_presence::PLAYER_PRESENCE,
                                crate::events::event::player_presence::Presence::new(
                                    data.player_name.clone(),
                                    String::from("joined"),
                                    game,
                                ),
                            ) {
                                error!("Failed to emit player presence event: {:?}", e);
                            }
                        }
                    }
                    ConnectionEventType::Disconnected => {
                        self.player_presence.remove(&data.player_name);
                        self.player_presence_debounce.remove(&data.player_name);

                        if let Err(e) = tauri::Emitter::emit(
                            &self.app_handle,
                            crate::events::event::player_presence::PLAYER_PRESENCE,
                            crate::events::event::player_presence::Presence::new(
                                data.player_name.clone(),
                                String::from("disconnected"),
                                game,
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

    // Process channel events (create, delete, join, leave)
    async fn handle_channel_event(&self, data: &QuicNetworkPacket) {
        let channel_event: Result<ChannelEventPacket, ()> = data.data.to_owned().try_into();

        match channel_event {
            Ok(event) => {
                let event_type = match event.event {
                    common::structs::channel::ChannelEvents::Create => "create",
                    common::structs::channel::ChannelEvents::Delete => "delete",
                    common::structs::channel::ChannelEvents::Join => "join",
                    common::structs::channel::ChannelEvents::Leave => "leave",
                    common::structs::channel::ChannelEvents::Rename => "rename",
                };

                info!(
                    "Channel event: {} {} in channel {} ({})",
                    event.name,
                    event_type,
                    event.channel,
                    event.channel_name.as_deref().unwrap_or("unknown")
                );

                if let Err(e) = tauri::Emitter::emit(
                    &self.app_handle,
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

    /// The first frame of another player's voice to reach this client.
    ///
    /// The last step of the install funnel, and the only one that cannot be reached by
    /// clicking through the UI: everything before it says a user arrived somewhere,
    /// this says voice actually worked end to end for them. It carries the game the
    /// speaker is playing and nothing that identifies either party.
    fn report_activation(&self, game: Option<String>) {
        if ACTIVATION_REPORTED.swap(true, Ordering::SeqCst) {
            return;
        }

        use tauri::Manager;
        let Some(analytics) = self
            .app_handle
            .try_state::<Arc<crate::analytics::AnalyticsService>>()
        else {
            return;
        };

        let data = game.map(|game| AnalyticsEventData::new().insert("game", game));
        analytics.track(AnalyticsEvent::Activated, data);
    }

    /// Processes AudioFramePacket data
    async fn handle_audio_data(&self, data: &QuicNetworkPacket) {
        let current_player_name = match self.metadata.get("current_player").await {
            Some(name) => name,
            None => return,
        };

        // Check if this is a new player we haven't seen before
        if let Some(sender) = &data.sender {
            let player_name = &sender.identity;

            // Skip presence tracking for synthetic jukebox players
            if !player_name.starts_with(common::consts::audio::JUKEBOX_PLAYER_PREFIX) {
                // Tell the gain projection which player this device belongs to. Every frame,
                // not just the first: a reconnect mints a new device id, and the projection
                // has to learn it before the next lookup rather than after a store write.
                if let Some(device) = sender.device {
                    self.gain.observe(device, player_name);
                }

                // Don't emit events for ourselves
                if !player_name.eq(&current_player_name) && !player_name.is_empty() {
                    // Resolve game from player_data cache, or preserve existing value in presence cache
                    let game = self
                        .players
                        .get(player_name)
                        .map(|p| p.get_game().as_str().to_string())
                        .or_else(|| self.player_presence.get(player_name).flatten());

                    // Always update the presence cache (stores game type alongside presence)
                    self.player_presence
                        .insert(player_name.clone(), game.clone());

                    // Voice arrived. Reported here rather than beside the presence
                    // event below because the debounce that guards presence is about
                    // not spamming the UI with the same player, and activation must not
                    // be lost to it.
                    self.report_activation(game.clone());

                    // Only emit if not recently debounced
                    if self.player_presence_debounce.get(player_name).is_none() {
                        self.player_presence_debounce
                            .insert(player_name.clone(), ());

                        // Emit synthetic presence event for new player detected via audio
                        if let Err(e) = tauri::Emitter::emit(
                            &self.app_handle,
                            crate::events::event::player_presence::PLAYER_PRESENCE,
                            crate::events::event::player_presence::Presence::new(
                                player_name.clone(),
                                String::from("joined"),
                                game,
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

        let sender = data.sender.clone();
        let data: Result<AudioFramePacket, ()> = data.data.to_owned().try_into();

        match data {
            Ok(data) => {
                #[cfg(feature = "bedrock-protocol")]
                if let Some(beacon_cache) = self.beacon_cache.as_ref() {
                    for meta in &data.metadata {
                        match meta {
                            AudioFrameMetadata::Jukebox(jb) => {
                                beacon_cache.observe(
                                    (&jb.position).into(),
                                    jb.dimension.clone(),
                                    &jb.event_id,
                                );
                            }
                        }
                    }
                }
                // Create emitter RecordingPlayerData from the server-stamped sender
                let emitter = sender
                    .as_ref()
                    .map(|s| {
                        RecordingPlayerData::from_packet_sender(
                            s,
                            &data,
                            self.player_gain_cache.get(&s.identity),
                        )
                    })
                    .unwrap_or_else(RecordingPlayerData::unknown);

                // Create listener RecordingPlayerData from current player
                let listener = self
                    .players
                    .get(&current_player_name)
                    .map(|p| {
                        RecordingPlayerData::from_player_enum(
                            &p,
                            current_player_name.clone(),
                            self.player_gain_cache.get(&current_player_name),
                        )
                    })
                    .unwrap_or_else(|| RecordingPlayerData::unknown());

                let timestamp = data.timestamp() as u64;
                let encoded_packet = EncodedAudioFramePacket {
                    timestamp: timestamp,
                    sample_rate: data.sample_rate,
                    data: data.data,
                    route: AudioSinkType::from_spatial(match data.spatial {
                        Some(s) => s,
                        None => true,
                    }),
                    emitter,
                    listener,
                    buffer_size_ms: 120,
                    time_between_reports_secs: 30,
                };

                // Send to playback - recording is now handled post-jitter-buffer in JitterBufferSource
                #[cfg(feature = "e2e")]
                crate::testkit::counters::TransportCounters::increment_into_jitter_buffer();
                match self.producer.send(encoded_packet.clone()) {
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
    async fn handle_player_data(&self, data: &QuicNetworkPacket) {
        let data: Result<PlayerDataPacket, ()> = data.data.to_owned().try_into();
        match data {
            Ok(data) => {
                let current_player_name = self.metadata.get("current_player").await;

                for player in data.players {
                    let player_name = player.get_name().to_string();

                    // The client's own world, which nothing else surfaces. It arrives on every
                    // pulse, so a mid-session transfer re-targets chat without any extra
                    // signal — and the webview cannot learn it any other way, because the
                    // position feed deliberately carries no world.
                    if current_player_name.as_deref() == Some(player_name.as_str()) {
                        if let common::PlayerEnum::Minecraft(mc) = &player {
                            let _ = tauri::Emitter::emit(
                                &self.app_handle,
                                "chat-world",
                                &mc.world_uuid,
                            );
                        }
                    }

                    self.players.insert(player_name, player);
                }
            }
            Err(_) => {
                warn!("Could not decode player data packet");
            }
        }
    }

    /// Server-relayed in-game chat, net mode.
    ///
    /// Shaped to match what the no-net proxy path emits so the webview has one listener and
    /// one line format regardless of which implementation is live.
    async fn handle_chat_message(&self, data: &QuicNetworkPacket) {
        let packet: Result<ChatMessagePacket, ()> = data.data.to_owned().try_into();
        let Ok(packet) = packet else {
            warn!("Could not decode chat message packet");
            return;
        };

        let payload = serde_json::json!({
            "author": packet.author,
            "text": packet.text,
            "system": packet.author.is_none(),
        });

        if let Err(e) = tauri::Emitter::emit(&self.app_handle, "bedrock-chat", payload) {
            warn!("Failed to emit chat line: {:?}", e);
        }
    }

    async fn handle_server_error(&self, data: &QuicNetworkPacket) {
        if let Ok(error_packet) = TryInto::<ServerErrorPacket>::try_into(data.data.clone()) {
            match error_packet.error_type {
                ServerErrorType::VersionIncompatible {
                    ref client_version,
                    ref server_version,
                } => {
                    error!(
                        "Protocol version mismatch: client={}, server={}",
                        client_version, server_version
                    );
                    let health_event = ConnectionHealth::VersionMismatch {
                        client_version: client_version.clone(),
                        server_version: server_version.clone(),
                        client_too_old: true,
                    };
                    info!("Publishing connection health: {:?}", health_event);
                    crate::network::HealthPublisher::publish(&self.app_handle, health_event);
                }
            }
        }
    }
}
