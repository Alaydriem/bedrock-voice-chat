use crate::AudioPacket;
use crate::AudioStreamManager;
use crate::NetworkPacket;
use crate::NetworkStreamManager;
use crate::RecordingManager;
use crate::audio::stream::stream_manager::sink::AudioOutputSink;
#[cfg(feature = "e2e")]
use crate::audio::stream::stream_manager::sink::CapturingSink;
use crate::audio::stream::stream_manager::source::AudioInputSource;
#[cfg(feature = "e2e")]
use crate::audio::stream::stream_manager::source::BridgeInputSource;
use crate::{Receiver, Sender};
use std::sync::Arc;
use tauri::Manager;
use tauri::async_runtime::Mutex;

/// Selects the audio backend used when wiring the managed application state.
/// `Real` selects the production Cpal input / Rodio output backends; `Fake`
/// injects a bridge input source and a capturing sink for the test harness so
/// the same construction path is exercised in both the real `run()` and tests.
pub enum AudioBackend {
    Real,
    #[cfg(feature = "e2e")]
    Fake {
        input: BridgeInputSource,
        capture: CapturingSink,
    },
}

/// Shared application-state construction extracted from `run()`'s setup. Creates
/// the audio/network flume channels, the RecordingManager, the
/// AudioStreamManager, the WebSocket and audio-actions managers, and the
/// NetworkStreamManager, then `.manage()`s each on the app. Desktop-only plugin
/// registrations (updater/global-shortcut/deep-link/single-instance/dialog)
/// stay in `run()` and are intentionally not performed here.
#[allow(clippy::too_many_arguments)]
pub fn build_managed_state(
    app: &tauri::App,
    backend: AudioBackend,
    #[cfg(feature = "bedrock-protocol")] player_state_cache: Option<
        Arc<crate::bedrock::BedrockPlayerStateCache>,
    >,
    #[cfg(feature = "bedrock-protocol")] beacon_cache: Option<Arc<crate::bedrock::JukeboxBeaconCache>>,
    #[cfg(feature = "bedrock-protocol")] eject_injector: Option<
        Arc<crate::bedrock::JukeboxEjectInjector>,
    >,
    #[cfg(feature = "bedrock-protocol")] presence_injector: Option<
        Arc<crate::bedrock::PresenceInjector>,
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
        Some(handle.state::<Arc<Mutex<RecordingManager>>>().inner().clone()),
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
