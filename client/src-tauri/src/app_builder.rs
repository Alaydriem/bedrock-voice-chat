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
        #[cfg(feature = "bedrock-protocol")] presence_injector: Option<
            Arc<crate::bedrock::PresenceInjector>,
        >,
        #[cfg(feature = "bedrock-protocol")] announce_injector: Option<
            Arc<crate::bedrock::AnnounceInjector>,
        >,
    ) -> anyhow::Result<()> {
        let handle = app.handle().clone();

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
            #[cfg(feature = "bedrock-protocol")]
            presence_injector,
            #[cfg(feature = "bedrock-protocol")]
            announce_injector,
        );
        app.manage(Mutex::new(audio_stream));

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

        let network_stream = NetworkStreamManager::new(
            handle.state::<Arc<Sender<AudioPacket>>>().inner().clone(),
            handle
                .state::<Arc<Receiver<NetworkPacket>>>()
                .inner()
                .clone(),
            handle.clone(),
        );
        app.manage(Mutex::new(network_stream));

        Ok(())
    }
}
