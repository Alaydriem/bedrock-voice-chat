use tauri::Manager;
use tauri::async_runtime::Mutex;

use crate::AudioStreamManager;
use crate::NetworkStreamManager;
use crate::analytics::AnalyticsService;
use crate::audio::types::AudioDeviceType;
use crate::structs::app_state::AppState;
use common::structs::channel::{ChannelEvent, ChannelEvents};
use std::sync::Arc;

// Resolved configuration the e2e harness uses to drive the connect sequence.
// Populated from the `BVC_E2E_*` environment variables by the bin; absent in
// the standalone boot smoke.
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub server: String,
    pub gamertag: String,
    pub code: String,
    // Channel display name — Connector calls create_channel and joins the returned id.
    pub channel: Option<String>,
    // Pre-existing channel id — Connector skips create_channel and joins directly.
    // Takes precedence over `channel` when both are set.
    pub channel_id: Option<String>,
}

// Drives the full client connect sequence against a live server by invoking the
// same Tauri command functions the production UI calls, so the harness never
// diverges from real client behavior. Lives in the library (not the bin) so it
// can reach the `pub(crate)` command layer.
pub struct Connector;

impl Connector {
    // Builds a telemetry-disabled, provider-free AnalyticsService for the e2e
    // bin to `.manage()`. The production `run()` manages one of these; the
    // connect commands (`change_network_stream`) read it from State, so the
    // harness must register an equivalent no-op instance to drive the real
    // command path without diverging.
    pub fn analytics_service() -> Arc<AnalyticsService> {
        let telemetry = Arc::new(crate::logging::Telemetry::new(false));
        Arc::new(AnalyticsService::new(
            telemetry,
            crate::analytics::PlatformId::new_shared(
                "00000000-0000-0000-0000-000000000000".to_string(),
            ),
        ))
    }

    /// Whether this process was told to run its voice session over WebSocket.
    ///
    /// Read from the environment because each e2e client is its own process, which is the
    /// only place a per-client transport choice can be expressed.
    fn websocket_is_forced() -> bool {
        std::env::var("BVC_E2E_FORCE_WEBSOCKET")
            .map(|value| value == "1")
            .unwrap_or(false)
    }

    // code_login -> initialize_api_client -> change_network_stream (QUIC) ->
    // join channel via api_channel_event. Returns the joined channel id (when a
    // channel was requested) once the QUIC stream is up and the join has been
    // attempted, so the caller can later target that id with explicit
    // leave/rejoin/delete operations.
    pub async fn run(
        handle: &tauri::AppHandle,
        config: &ConnectConfig,
    ) -> Result<Option<String>, String> {
        let login =
            crate::auth::code_login::code_login(config.server.clone(), config.code.clone())
                .await
                .map_err(|_| "code login failed".to_string())?;

        let app_state = handle.state::<Mutex<AppState>>();
        {
            let mut state = app_state.lock().await;
            state
                .initialize_api_client(
                    config.server.clone(),
                    login.certificate_ca.clone(),
                    login.certificate.clone() + &login.certificate_key.clone(),
                )
                .await;
            state.current_server = Some(config.server.clone());
        }

        // Places this client on the WebSocket transport by writing the verdict a client
        // reaches in the field once QUIC has degraded on a host. Selection then takes the
        // real demoted branch, so the scenario exercises production code rather than a
        // harness-only path around it. Without this, transport is a property of the server
        // config and every client against one server shares it — which cannot express a
        // mixed-transport channel.
        if Self::websocket_is_forced() {
            handle
                .state::<Mutex<NetworkStreamManager>>()
                .lock()
                .await
                .transport_verdict()
                .demote(&config.server);
        }

        // Bring up the QUIC stream through the production command rather than a
        // private reimplementation so DNS resolution and restart stay in sync.
        crate::commands::network::change_network_stream(
            handle.clone(),
            config.server.clone(),
            login.clone(),
            handle.state::<Mutex<AppState>>(),
            handle.state::<Mutex<NetworkStreamManager>>(),
            handle.state::<Arc<AnalyticsService>>(),
        )
        .await?;

        // Seed `current_player` into the output stream's metadata cache, then
        // start both audio streams. In the production app this is done by
        // `update_current_player` + `change_audio_device` Tauri commands.
        // The testkit never invokes those commands, so we replicate the
        // minimal two-step here for both the input and output streams.
        let asm = handle.state::<Mutex<AudioStreamManager>>();
        {
            let mut asm = asm.lock().await;
            let _ = asm
                .metadata(
                    "current_player".to_string(),
                    login.gamertag.clone(),
                    &AudioDeviceType::OutputDevice,
                )
                .await;
            if let Err(e) = asm.start(AudioDeviceType::OutputDevice).await {
                log::warn!("testkit: output stream start failed: {e}");
            }
            if let Err(e) = asm.start(AudioDeviceType::InputDevice).await {
                log::warn!("testkit: input stream start failed: {e}");
            }
        }

        let join_id: Option<String> = if let Some(id) = &config.channel_id {
            // Pre-existing channel id supplied directly: skip creation.
            Some(id.clone())
        } else if let Some(channel_name) = &config.channel {
            // Try to find an existing channel by name first; create only when none exists.
            // This lets multiple clients in the same test join the same channel by name
            // without each creating a separate one.
            let existing_id =
                crate::api::commands::api_list_channels(handle.state::<Mutex<AppState>>(), None)
                    .await
                    .ok()
                    .and_then(|channels| {
                        channels
                            .into_iter()
                            .find(|c| c.name == *channel_name)
                            .map(|c| c.id())
                    });

            if let Some(id) = existing_id {
                Some(id)
            } else {
                // The server assigns a random id on create; a Join targets that id,
                // not the display name, and 400s on a missing channel. Create the
                // channel first (the same order the production client follows) and
                // join the returned id.
                Some(
                    crate::api::commands::api_create_channel(
                        handle.state::<Mutex<AppState>>(),
                        channel_name.clone(),
                        None,
                    )
                    .await?,
                )
            }
        } else {
            None
        };

        if let Some(channel_id) = &join_id {
            crate::api::commands::api_channel_event(
                handle.state::<Mutex<AppState>>(),
                channel_id.clone(),
                ChannelEvent::new(ChannelEvents::Join),
                None,
            )
            .await?;
        }

        Ok(join_id)
    }

    // Drives one explicit channel-membership operation through the production
    // `api_channel_event` path against an already-joined channel id. Used by the
    // harness to exercise leave / rejoin / disband without a reconnect.
    pub async fn channel_event(
        handle: &tauri::AppHandle,
        channel_id: String,
        event: ChannelEvents,
    ) -> Result<(), String> {
        crate::api::commands::api_channel_event(
            handle.state::<Mutex<AppState>>(),
            channel_id,
            ChannelEvent::new(event),
            None,
        )
        .await
        .map(|_| ())
    }

    // Uploads an audio file through the production client path: encode (decode →
    // mono → 48k → Opus → Ogg via AudioFileEncoder) then POST /api/audio/file
    // against the connected, authenticated server. Returns the AudioFileResponse.
    pub async fn upload_audio(
        handle: &tauri::AppHandle,
        wav_path: String,
        game: String,
    ) -> Result<common::response::AudioFileResponse, String> {
        crate::commands::audio_library::upload_audio_file(
            handle.state::<Mutex<AppState>>(),
            wav_path,
            None,
            Some(game),
        )
        .await
    }

    // Gracefully tears down the QUIC connection via the same path the production
    // app uses when switching servers (`NetworkStreamManager::reset`). Aborting
    // the stream tasks and dropping the stream structs drops every
    // `Arc<Connection>` clone (the input/output streams and the health monitor
    // task) shortly after their aborts land, so s2n-quic emits a
    // CONNECTION_CLOSE once the refcount reaches zero and the server's disconnect
    // callback runs its cache + registry cleanup promptly — rather than waiting
    // out the idle-timeout recovery window.
    pub async fn disconnect(handle: &tauri::AppHandle) -> Result<(), String> {
        let nsm = handle.state::<Mutex<NetworkStreamManager>>();
        let mut nsm = nsm.lock().await;
        nsm.reset().await.map_err(|e| e.to_string())
    }

    // Starts the Bedrock proxy via the production `bedrock_start_proxy` command
    // path. Because proxy tests run against a fake upstream (not Xbox Live), a
    // stub `AuthManager` is seeded into `BedrockState` so the command's auth
    // guard passes; the stub carries no real credentials and authentication will
    // fail at the upstream connection level, which is expected and non-fatal for
    // the harness scenarios this is designed to support.
    //
    // `listen_port` is the local UDP port the proxy will bind. `upstream_host`
    // and `upstream_port` point at the fake `BedrockServer`. On success the proxy
    // is listening and `()` is returned; the caller emits `ProxyStarted`.
    #[cfg(feature = "bedrock-protocol")]
    pub async fn start_proxy(
        handle: &tauri::AppHandle,
        upstream_host: String,
        upstream_port: u16,
        listen_port: u16,
        addon_mode: Option<common::structs::bedrock::AddonMode>,
    ) -> Result<(), String> {
        use crate::bedrock::BedrockState;

        {
            let state = handle.state::<Mutex<BedrockState>>();
            let mut state = state.lock().await;
            if state.auth_manager.is_none() {
                let auth_manager =
                    std::sync::Arc::new(common::bedrock_protocol::AuthManager::offline());
                state.auth_manager = Some(auth_manager);
            }
        }

        crate::commands::bedrock::bedrock_start_proxy(
            handle.clone(),
            upstream_host,
            upstream_port,
            Some(listen_port),
            "127.0.0.1".to_string(),
            None,
            addon_mode,
        )
        .await
    }

    // Returns a freshly-constructed `BedrockState` wrapped in an async Mutex,
    // ready to be registered with `app.manage()` in the e2e bin.
    #[cfg(feature = "bedrock-protocol")]
    pub fn bedrock_state() -> Mutex<crate::bedrock::BedrockState> {
        Mutex::new(crate::bedrock::BedrockState::new())
    }

    // Returns a no-op `FeatureFlagService` (empty API key, no remote refresh)
    // for the e2e bin to register so `bedrock_start_proxy` can extract it from
    // State without hitting a real feature-flag endpoint.
    #[cfg(feature = "bedrock-protocol")]
    pub fn feature_flag_service() -> Arc<crate::feature_flags::FeatureFlagService> {
        Arc::new(crate::feature_flags::FeatureFlagService::new(
            String::new(),
            String::new(),
            crate::analytics::PlatformId::new_shared(String::new()),
            0,
            std::time::Duration::from_secs(3600),
            None,
        ))
    }

    // Returns a freshly-constructed `JukeboxBeaconCache` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn beacon_cache() -> Arc<crate::bedrock::JukeboxBeaconCache> {
        Arc::new(crate::bedrock::JukeboxBeaconCache::new())
    }

    // Returns a freshly-constructed `JukeboxEjectInjector` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn eject_injector() -> Arc<crate::bedrock::JukeboxEjectInjector> {
        crate::bedrock::JukeboxEjectInjector::new_shared()
    }

    // Returns a freshly-constructed `PresenceInjector` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn presence_injector() -> Arc<crate::bedrock::PresenceInjector> {
        crate::bedrock::PresenceInjector::new_shared()
    }

    // Returns a freshly-constructed `AnnounceInjector` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn announce_injector() -> Arc<crate::bedrock::AnnounceInjector> {
        crate::bedrock::AnnounceInjector::new_shared()
    }

    // Returns a freshly-constructed `BedrockConnectErrorChannel` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn connect_error_channel() -> Arc<crate::bedrock::BedrockConnectErrorChannel> {
        Arc::new(crate::bedrock::BedrockConnectErrorChannel::new())
    }

    // Returns a freshly-constructed `BedrockChatChannel` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn chat_channel() -> Arc<crate::bedrock::BedrockChatChannel> {
        Arc::new(crate::bedrock::BedrockChatChannel::new())
    }

    // Returns a freshly-constructed `ChatInjector` for state registration.
    #[cfg(feature = "bedrock-protocol")]
    pub fn chat_injector() -> Arc<crate::bedrock::ChatInjector> {
        crate::bedrock::ChatInjector::new_shared()
    }
}
