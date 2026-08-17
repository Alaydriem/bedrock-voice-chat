use crate::AudioPacket;
use crate::AudioStreamManager;
use crate::NetworkPacket;
use crate::NetworkStreamManager;
use crate::RecordingManager;
use crate::audio::AudioBackend;
use crate::audio::stream::stream_manager::sink::AudioOutputSink;
use crate::audio::stream::stream_manager::source::AudioInputSource;
use crate::{Receiver, Sender};
use std::sync::Arc;
use tauri::Manager;
use tauri::async_runtime::Mutex;

/// Shared application-state construction used by both the real `run()` setup
/// and the e2e harness bin, so the two never diverge on wiring.
pub struct AppBuilder;

impl AppBuilder {
    /// Creates the audio/network flume channels, the RecordingManager, the
    /// AudioStreamManager, the WebSocket and audio-actions managers, the
    /// control plane, and the NetworkStreamManager, then `.manage()`s each on
    /// the app. Desktop-only plugin registrations (updater/global-shortcut/
    /// deep-link/single-instance/dialog) stay in `run()` and are intentionally
    /// not performed here.
    #[allow(clippy::too_many_arguments)]
    pub fn build_managed_state(
        app: &tauri::App,
        backend: AudioBackend,
        #[cfg(feature = "bedrock-protocol")] player_state_cache: Option<
            Arc<crate::bedrock::BedrockPlayerStateCache>,
        >,
        #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<
            Arc<crate::bedrock::JukeboxBeaconCache>,
        >,
        #[cfg(feature = "bedrock-protocol")] eject_injector: Option<
            Arc<crate::bedrock::JukeboxEjectInjector>,
        >,
    ) -> anyhow::Result<()> {
        let handle = app.handle().clone();

        // Absent packs are not an error: message ids are the English source strings, so a
        // directory that does not exist leaves the app in English rather than broken.
        let resource_dir = app
            .path()
            .resource_dir()
            .map(|dir| dir.join("resources").join("i18n"))
            .unwrap_or_default();
        app.manage(crate::i18n::LocalizationService::new(resource_dir));

        // This is our audio producer and consumer
        // The producer is responsible for getting audio from the raw input device, then sending it to the consumer
        // The consumer lives in the networking thread, consumes the audio, then sends it to the server
        let (audio_producer, audio_consumer) = flume::bounded::<AudioPacket>(10000);
        app.manage(Arc::new(audio_producer));
        app.manage(Arc::new(audio_consumer));

        // This is our network producer and consumer
        // The producer retrieves data from the raw QUIC stream, then sends it to the consumer
        // The consumer receives the data, then pushed it to the output device
        let (quic_producer, quic_consumer) = flume::bounded::<NetworkPacket>(10000);
        app.manage(Arc::new(quic_producer));
        app.manage(Arc::new(quic_consumer));

        // This is our RecordingManager
        // It is responsible for managing recording sessions and owns internal producer/consumer channels
        // for both the input and output stream
        let recording_manager = RecordingManager::new(handle.clone());
        app.manage(Arc::new(Mutex::new(recording_manager)));

        // Translate the requested backend into the concrete source/sink the manager
        // consumes into its initial streams. `Real` is identical to today's wiring.
        let (input_source, output_sink) = match backend {
            AudioBackend::Real => (AudioInputSource::Cpal, AudioOutputSink::Rodio),
            #[cfg(feature = "e2e")]
            AudioBackend::Fake { input, capture } => (
                AudioInputSource::Fake(input),
                AudioOutputSink::Fake(capture),
            ),
        };

        let audio_stream = AudioStreamManager::new_with_sources(
            handle.state::<Arc<Sender<NetworkPacket>>>().inner().clone(),
            handle.state::<Arc<Receiver<AudioPacket>>>().inner().clone(),
            handle.clone(),
            Some(
                handle
                    .state::<Arc<Mutex<RecordingManager>>>()
                    .inner()
                    .clone(),
            ),
            input_source,
            output_sink,
            #[cfg(feature = "bedrock-protocol")]
            player_state_cache,
            #[cfg(feature = "bedrock-protocol")]
            beacon_cache,
            #[cfg(feature = "bedrock-protocol")]
            eject_injector,
        );

        // Pulled out before the manager is moved behind its mutex, so the diagnostics service
        // never has to take that lock to read a counter — it is contended with playback.
        let input_stats = audio_stream.input_stats();
        let peer_registry = audio_stream.peer_registry();
        // Owned by the audio stream, because that is what writes into it. Taken from there rather
        // than constructed here so the writer and the diagnostic share one instance, and pulled out
        // before the move for the same reason as the two above.
        let session_config = audio_stream.session_config();
        let level_bus = audio_stream.levels();
        // Read by the runtime-state poll, which must not take the audio manager's lock to learn
        // that a rebuild gave up — the rebuild itself holds that lock while it runs.
        let capture_availability = audio_stream.capture_availability();
        // Read by every mute and deafen surface, which must not take the audio manager's lock
        // to play a tone — the action that triggered the cue is already holding it.
        let cue_sink = audio_stream.cue_sink();

        app.manage(Mutex::new(audio_stream));
        app.manage(capture_availability);
        app.manage(cue_sink);

        // Per-player volume and mute. Registered here rather than in `run()` because both the
        // desktop app and the e2e harness need it: the in-game control actions and the
        // control plane's preference report both resolve it out of managed state, and a
        // binary without it silently applies no volumes at all.
        //
        // Kept out of `store.json` because that file holds the auth token and the server
        // list, and `save()` rewrites all of it — so a player walking into earshot used to
        // rewrite the token to disk.
        // `BVC_PLAYER_SETTINGS_PATH` overrides the location. The e2e harness sets it per client,
        // because every harness process shares one app identifier and therefore one path —
        // redb takes an exclusive lock, so exactly one of a scenario's clients would get a real
        // store and the rest would silently fall back to memory, nondeterministically.
        let player_settings = match std::env::var_os("BVC_PLAYER_SETTINGS_PATH")
            .map(|path| Ok(std::path::PathBuf::from(path)))
            .unwrap_or_else(|| {
                handle
                    .path()
                    .app_local_data_dir()
                    .map(|dir| dir.join("player_settings.redb"))
            }) {
            Ok(path) => match crate::players::RedbBackend::open(&path) {
                Ok(backend) => crate::players::PlayerSettingsService::new_shared(
                    crate::players::PlayerSettings::Redb(backend),
                ),
                Err(cause) => {
                    // Transient: locked, no permission, disk full. Run in memory so audio
                    // still applies what the user sets this session, and leave the file alone
                    // so nothing is lost once the condition clears.
                    log::error!(
                        "Player settings unavailable, running in memory this session: {cause}"
                    );
                    crate::players::PlayerSettingsService::new_memory_only()
                }
            },
            Err(cause) => {
                log::error!("No local data directory for player settings: {cause}");
                crate::players::PlayerSettingsService::new_memory_only()
            }
        };
        player_settings.clone().spawn_debounce();
        app.manage(crate::players::PlayerSettingsCoordinator::new_shared(
            player_settings.clone(),
        ));
        app.manage(player_settings);

        // Initialize WebSocketManager and register the broadcaster
        let ws_manager = crate::websocket::WebSocketManager::new(handle.clone());
        let ws_broadcaster = ws_manager.broadcaster();
        app.manage(ws_broadcaster);
        app.manage(Mutex::new(ws_manager));

        // AudioActionsManager handles mute, deafen, and recording state changes for both user-initiated actions (keybinds) and API calls
        let audio_actions = crate::audio::AudioActionsManager::new(handle.clone());
        app.manage(audio_actions);

        // Control-action plane: proxy sessions and the QUIC output router push
        // delivered ClientActions into this channel; the single consumer owns the
        // ControlActionsManager (and with it the AppHandle) and applies them against
        // the managers. Producers only ever hold the sender.
        let (control_tx, control_rx) = crate::control::ControlActionSender::channel();
        app.manage(control_tx);
        let control_actions = crate::control::ControlActionsManager::new(handle.clone());
        tauri::async_runtime::spawn(async move {
            control_actions.run(control_rx).await;
        });

        // Control-state reporting: managers signal local audio-state / preference
        // changes on this bus; the QueryStateReporter debounces them and pushes
        // ServerBound QueryState / PlayerPreference packets so the server's control
        // cache mirrors the client. The identity is published by the
        // NetworkStreamManager when a QUIC stream comes up.
        app.manage(crate::control::ConnectionIdentity::new_shared());
        app.manage(crate::chat::ChatPolicy::new_shared());
        let state_bus = crate::control::ControlStateBus::new();
        let state_rx = state_bus.subscribe();
        app.manage(state_bus);
        let reporter = crate::control::QueryStateReporter::new(handle.clone());
        tauri::async_runtime::spawn(async move {
            reporter.run(state_rx).await;
        });

        // No-net reverse ride: the reporter enqueues encoded !bvcs: state messages
        // here; a running proxy session drains them into serverbound chat.
        #[cfg(feature = "bedrock-protocol")]
        app.manage(crate::bedrock::QueryStateInjector::new_shared());

        // Link diagnostics. The QUIC stats handle travels on a watch channel because a
        // reconnect mints a fresh one, and the service must follow the live connection rather
        // than hold a handle to a dead one.
        let transport_stats = Arc::new(crate::diagnostics::TransportStats::new());
        let link_session = Arc::new(crate::diagnostics::LinkSession::new());
        let (quic_stats_tx, quic_stats_rx) =
            tokio::sync::watch::channel(Arc::new(crate::diagnostics::QuicLinkStats::new()));

        // The e2e harness reads its transport-fidelity counters through this same instance, so
        // the numbers the tests trust and the numbers a player sees cannot diverge.
        #[cfg(feature = "e2e")]
        crate::testkit::counters::TransportCounters::register(transport_stats.clone());

        let network_stream = NetworkStreamManager::new(
            handle.state::<Arc<Sender<AudioPacket>>>().inner().clone(),
            handle
                .state::<Arc<Receiver<NetworkPacket>>>()
                .inner()
                .clone(),
            handle.clone(),
            quic_stats_tx,
            link_session.clone(),
            transport_stats.clone(),
        );
        app.manage(Mutex::new(network_stream));

        let device_info = Arc::new(crate::diagnostics::DeviceInfo::new());
        // Resolved spatial settings are recorded where the audio pipeline reads them, so a
        // report describes what this session runs under rather than the defaults.
        app.manage(session_config.clone());
        app.manage(device_info.clone());

        let diagnostics = crate::diagnostics::LinkDiagnosticsService::new_shared(
            quic_stats_rx,
            transport_stats,
            input_stats,
            link_session,
            session_config,
            peer_registry,
            device_info,
            level_bus,
        );
        diagnostics.clone().start(handle.clone());
        app.manage(diagnostics);

        Ok(())
    }
}
