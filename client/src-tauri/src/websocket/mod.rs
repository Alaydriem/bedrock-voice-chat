use common::structs::keybinds::VoiceMode as KeybindVoiceMode;
use common::structs::network::ConnectionHealth;
use common::traits::StreamTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::{broadcast, watch};
use tokio::task::AbortHandle;

pub mod clients;
pub mod route;
pub mod structs;

pub use clients::{ClientRegistration, WebSocketClients};
pub use route::{RejectReason, WebSocketRoute};
pub use structs::{
    Command, CommandMessage, ConnectData, ConnectTarget, ConnectTargetKind, DeviceType,
    ErrorResponse, GroupData, JukeboxData, MuteData, PongData, PttData, RecordData, ResponseData,
    StateData, SuccessResponse, TargetsData, VoiceMode, VoiceModeGuard,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub localhost_only: bool,
    pub port: u16,
    pub key: String,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            // Loopback on desktop; a phone has nothing local to serve.
            localhost_only: !cfg!(mobile),
            port: 9595,
            key: String::new(),
        }
    }
}

/// Wrapper around a broadcast sender for sharing with Tauri managed state.
/// UI commands (mute, recording) use this to push state updates to all connected WS clients.
pub struct WebSocketBroadcaster {
    pub commands: broadcast::Sender<String>,
    // A separate channel so a one-per-second diagnostics push cannot lag a command subscriber out
    // of its buffer, and so a command client that never asked for metrics is not sent any.
    pub metrics: broadcast::Sender<String>,
    // Retained rather than only broadcast, so a subscriber that upgrades mid-session is told the
    // current state instead of waiting for a transition. A healthy client produces none for
    // hours, and a failed one produces no metrics frames to infer anything from.
    health: watch::Sender<ConnectionHealth>,
}

impl WebSocketBroadcaster {
    /// Serialize a diagnostics snapshot and broadcast it to `/metrics` subscribers.
    ///
    /// The envelope is tagged. `ResponseData` is `#[serde(untagged)]`, so a consumer could not
    /// distinguish a metrics frame from a state frame by shape alone if this rode on that enum.
    pub fn broadcast_metrics(&self, snapshot: common::structs::metrics::LinkDiagnosticsSnapshot) {
        let push = common::structs::metrics::MetricsPush::new(snapshot);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.metrics.send(json);
        }
    }

    /// Broadcast a connection-health verdict to `/metrics` subscribers, and retain it for
    /// whoever subscribes next.
    ///
    /// Rides the metrics channel rather than the command channel: it describes the link a metrics
    /// subscriber is measuring, and a command client has `state` for what it cares about.
    pub fn broadcast_health(&self, health: ConnectionHealth) {
        let push = common::structs::metrics::HealthPush::new(health.clone());
        let _ = self.health.send(health);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.metrics.send(json);
        }
    }

    /// The last verdict published, for a subscriber that has just arrived.
    pub fn latest_health(&self) -> ConnectionHealth {
        self.health.borrow().clone()
    }

    /// Serialize a StateData DTO and broadcast to all connected WS clients.
    pub fn broadcast_state(&self, state: StateData) {
        let response = SuccessResponse::state(state);
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.commands.send(json);
        }
    }
}

pub struct WebSocketManager {
    abort_handle: Option<AbortHandle>,
    shutdown_tx: Option<watch::Sender<bool>>,
    config: Option<WebSocketConfig>,
    app_handle: AppHandle,
    broadcast_tx: broadcast::Sender<String>,
    metrics_tx: broadcast::Sender<String>,
    health_tx: watch::Sender<ConnectionHealth>,
    clients: Arc<WebSocketClients>,
}

impl WebSocketManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let config: Option<WebSocketConfig> = app_handle
            .store("store.json")
            .ok()
            .and_then(|store| store.get("websocket_server"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let (broadcast_tx, _) = broadcast::channel(16);
        // Deeper than the command channel: this carries one frame per second, so a subscriber
        // that stalls briefly should fall behind rather than be dropped.
        let (metrics_tx, _) = broadcast::channel(64);
        // Disconnected until something says otherwise. `Connected` as the initial value would
        // tell the first subscriber the link is up before any connection has been attempted.
        let (health_tx, _) = watch::channel(ConnectionHealth::Disconnected);

        Self {
            abort_handle: None,
            shutdown_tx: None,
            config,
            app_handle,
            broadcast_tx,
            metrics_tx,
            health_tx,
            clients: WebSocketClients::new_shared(),
        }
    }

    /// Extract a broadcaster handle for registration as Tauri managed state
    pub fn broadcaster(&self) -> WebSocketBroadcaster {
        WebSocketBroadcaster {
            commands: self.broadcast_tx.clone(),
            metrics: self.metrics_tx.clone(),
            health: self.health_tx.clone(),
        }
    }

    /// The live connection registry, for the settings pane that lists them.
    pub fn clients(&self) -> Arc<WebSocketClients> {
        self.clients.clone()
    }

    pub fn update_config(&mut self, config: WebSocketConfig) {
        self.config = Some(config);
    }
}

impl StreamTrait for WebSocketManager {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.abort_handle.is_some() {
            return Err(anyhow::anyhow!("WebSocket server already running"));
        }

        // Pre-check: ensure we have valid config
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebSocket config not set"))?;

        if !config.enabled {
            return Err(anyhow::anyhow!("WebSocket server is not enabled"));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let handle = self.start_server_loop(shutdown_rx).await?;
        self.abort_handle = Some(handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        // Signal all active connections to shut down
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(task) = &self.abort_handle {
            task.abort();
        }

        self.abort_handle = None;
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.abort_handle.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        // Update config based on key-value pairs if needed
        Ok(())
    }
}

impl WebSocketManager {
    async fn start_server_loop(
        &self,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<AbortHandle, anyhow::Error> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config available"))?;

        let host = if config.localhost_only {
            "127.0.0.1"
        } else {
            "0.0.0.0"
        };
        let addr = format!("{}:{}", host, config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let config = config.clone();
        let app_handle = self.app_handle.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        let metrics_tx = self.metrics_tx.clone();
        let clients = self.clients.clone();

        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app_handle = app_handle.clone();
                let key = config.key.clone();
                let broadcast_tx = broadcast_tx.clone();
                let metrics_tx = metrics_tx.clone();
                let shutdown_rx = shutdown_rx.clone();
                let clients = clients.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(
                        stream,
                        app_handle,
                        key,
                        broadcast_tx,
                        metrics_tx,
                        shutdown_rx,
                        clients,
                    )
                    .await
                    {
                        // Connection resets / broken pipes are normal client disconnects
                        let is_disconnect = e.root_cause().downcast_ref::<std::io::Error>().map_or(
                            false,
                            |io_err| {
                                matches!(
                                    io_err.kind(),
                                    std::io::ErrorKind::ConnectionReset
                                        | std::io::ErrorKind::ConnectionAborted
                                        | std::io::ErrorKind::BrokenPipe
                                )
                            },
                        );

                        if !is_disconnect {
                            log::error!("Connection error: {}", e);
                        }
                    }
                });
            }
        });

        Ok(handle.abort_handle())
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        app_handle: AppHandle,
        auth_key: String,
        broadcast_tx: broadcast::Sender<String>,
        metrics_tx: broadcast::Sender<String>,
        shutdown_rx: watch::Receiver<bool>,
        clients: Arc<WebSocketClients>,
    ) -> Result<(), anyhow::Error> {
        use std::sync::Mutex as StdMutex;
        use tokio_tungstenite::accept_hdr_async;
        use tokio_tungstenite::tungstenite::handshake::server::{
            ErrorResponse as HandshakeError, Request, Response,
        };
        use tokio_tungstenite::tungstenite::http::StatusCode;

        // The route is decided during the handshake so a bad path or a wrong key refuses the
        // upgrade outright, rather than accepting a client that then waits forever.
        let resolved: Arc<StdMutex<Option<WebSocketRoute>>> = Arc::new(StdMutex::new(None));
        let captured = resolved.clone();
        let key_for_callback = auth_key.clone();

        // The only self-description a client offers, and the handshake is the only place
        // it is available.
        let agent: Arc<StdMutex<String>> = Arc::new(StdMutex::new(String::new()));
        let captured_agent = agent.clone();

        let callback = move |request: &Request, response: Response| {
            let uri = request.uri().to_string();
            if let Some(value) = request
                .headers()
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                && let Ok(mut slot) = captured_agent.lock()
            {
                *slot = value.to_string();
            }
            match WebSocketRoute::resolve(&uri, &key_for_callback) {
                Ok(route) => {
                    if let Ok(mut slot) = captured.lock() {
                        *slot = Some(route);
                    }
                    Ok(response)
                }
                Err(reason) => {
                    log::warn!("Rejected WebSocket upgrade: {}", reason.as_str());
                    let status = StatusCode::UNAUTHORIZED;
                    let _ = reason;
                    let mut error = HandshakeError::new(Some(reason.as_str().to_string()));
                    *error.status_mut() = status;
                    Err(error)
                }
            }
        };

        let ws_stream = accept_hdr_async(stream, callback).await?;
        let route = resolved
            .lock()
            .ok()
            .and_then(|slot| *slot)
            .unwrap_or(WebSocketRoute::Command);

        let name = agent.lock().map(|slot| slot.clone()).unwrap_or_default();
        // Held for the life of the connection. Dropping it releases the registration,
        // which is the only way to cover all three exits: an error, the shutdown watch,
        // and a peer that simply went away.
        let registration = ClientRegistration::new(clients, &name, route.as_str());

        match route {
            WebSocketRoute::Metrics => {
                let health = app_handle
                    .try_state::<WebSocketBroadcaster>()
                    .map(|broadcaster| broadcaster.latest_health())
                    .unwrap_or(ConnectionHealth::Disconnected);

                Self::serve_metrics(ws_stream, metrics_tx, shutdown_rx, health).await
            }
            WebSocketRoute::Command => {
                Self::serve_commands(
                    ws_stream,
                    app_handle,
                    auth_key,
                    broadcast_tx,
                    shutdown_rx,
                    &registration,
                )
                .await
            }
        }
    }

    // Push only. Inbound frames other than close and ping are ignored, because this endpoint has
    // no commands.
    async fn serve_metrics(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        metrics_tx: broadcast::Sender<String>,
        mut shutdown_rx: watch::Receiver<bool>,
        initial_health: ConnectionHealth,
    ) -> Result<(), anyhow::Error> {
        use futures_util::{SinkExt, StreamExt};

        let (mut write, mut read) = ws_stream.split();
        let mut metrics_rx = metrics_tx.subscribe();

        // Sent before anything is forwarded, so a subscriber knows the current state rather than
        // inferring it from an absence of frames. A healthy client produces no transition for
        // hours, and a failed one produces no metrics frames to read anything from at all.
        let push = common::structs::metrics::HealthPush::new(initial_health);
        if let Ok(json) = serde_json::to_string(&push) {
            write
                .send(tokio_tungstenite::tungstenite::Message::Text(json.into()))
                .await?;
        }

        loop {
            tokio::select! {
                frame = read.next() => {
                    match frame {
                        Some(Ok(msg)) if msg.is_close() => return Ok(()),
                        Some(Ok(_)) => continue,
                        Some(Err(e)) => return Err(e.into()),
                        None => return Ok(()),
                    }
                }

                result = metrics_rx.recv() => {
                    match result {
                        Ok(json) => {
                            write
                                .send(tokio_tungstenite::tungstenite::Message::Text(json.into()))
                                .await?;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("Metrics subscriber lagged by {} frames", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => return Ok(()),
                    }
                }

                _ = shutdown_rx.changed() => return Ok(()),
            }
        }
    }

    async fn serve_commands(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        app_handle: AppHandle,
        auth_key: String,
        broadcast_tx: broadcast::Sender<String>,
        mut shutdown_rx: watch::Receiver<bool>,
        registration: &ClientRegistration,
    ) -> Result<(), anyhow::Error> {
        use futures_util::{SinkExt, StreamExt};

        let (mut write, mut read) = ws_stream.split();
        let mut broadcast_rx = broadcast_tx.subscribe();

        loop {
            tokio::select! {
                // Branch 1: Read from WS client
                msg = read.next() => {
                    let msg = match msg {
                        Some(Ok(msg)) => msg,
                        Some(Err(e)) => return Err(e.into()),
                        None => return Ok(()), // client disconnected
                    };

                    if !msg.is_text() && !msg.is_binary() {
                        continue;
                    }

                    let text = msg.to_text()?;

                    // Parse message with optional key
                    let parsed = match CommandMessage::from_json(text) {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            let error_response = ErrorResponse::new(e.to_string());
                            let json = serde_json::to_string(&error_response)?;
                            write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await?;
                            continue;
                        }
                    };

                    // No configured key refuses everything rather than accepting everything.
                    if auth_key.is_empty() || parsed.key.as_deref() != Some(auth_key.as_str()) {
                        let error_response = ErrorResponse::new("Invalid authentication key".to_string());
                        let json = serde_json::to_string(&error_response)?;
                        write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await?;
                        continue;
                    }

                    // Counted once it has been accepted and authenticated. Counting every
                    // inbound frame would report rejected traffic as work done.
                    registration.count_command();

                    // Check if this is a state-changing command
                    let is_state_changing = matches!(
                        parsed.command,
                        Command::Mute { .. }
                            | Command::Record
                            | Command::Jukebox
                            | Command::JukeboxVolume { .. }
                            | Command::CreateGroup { .. }
                            | Command::JoinGroup { .. }
                            | Command::LeaveGroup
                    );

                    // Execute command
                    let response_json = match Self::execute_command_from(parsed.command, &app_handle).await {
                        Ok(data) => {
                            let success_response = SuccessResponse {
                                success: true,
                                data,
                            };
                            serde_json::to_string(&success_response)?
                        }
                        Err(e) => {
                            let error_response = ErrorResponse::new(e.to_string());
                            serde_json::to_string(&error_response)?
                        }
                    };

                    write.send(tokio_tungstenite::tungstenite::Message::Text(response_json.into())).await?;

                    // After a state-changing command, broadcast full state to all other clients
                    if is_state_changing {
                        if let Ok(state_json) = Self::build_state_json(&app_handle).await {
                            // Ignore send errors (no receivers is fine)
                            let _ = broadcast_tx.send(state_json);
                        }
                    }
                }

                // Branch 2: Read from broadcast channel (state updates from other clients or UI)
                result = broadcast_rx.recv() => {
                    match result {
                        Ok(json) => {
                            write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await?;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("WebSocket broadcast receiver lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            return Ok(());
                        }
                    }
                }

                // Branch 3: Server shutdown signal
                _ = shutdown_rx.changed() => {
                    return Ok(());
                }
            }
        }
    }

    async fn execute_command_from(
        cmd: Command,
        app_handle: &AppHandle,
    ) -> Result<ResponseData, anyhow::Error> {
        // One rule, asked once, from the crate a controller compiles against — rather than
        // a check per arm that a new command can forget to make.
        if let Some(reason) = VoiceModeGuard::refusal(Self::voice_mode(app_handle).await, &cmd) {
            return Err(anyhow::anyhow!(reason));
        }

        match cmd {
            Command::Ping => Ok(ResponseData::Pong(PongData { pong: true })),

            Command::Mute { device } => {
                let audio_device = match device {
                    DeviceType::Input => crate::audio::types::AudioDeviceType::InputDevice,
                    DeviceType::Output => crate::audio::types::AudioDeviceType::OutputDevice,
                };

                let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                let status = actions.toggle_mute(audio_device).await;

                let device_str = match device {
                    DeviceType::Input => "input",
                    DeviceType::Output => "output",
                };

                Ok(ResponseData::Mute(MuteData {
                    device: device_str.to_string(),
                    muted: status,
                }))
            }

            // A held key on a controller. The client owns the release: a connection that
            // drops mid-hold does not close the microphone.
            Command::Ptt { down } => {
                let listener = app_handle.state::<Arc<crate::keybinds::KeybindListener>>();
                listener.set_ptt(down).await;
                Ok(ResponseData::Ptt(PttData {
                    active: listener.is_ptt_held(),
                }))
            }

            Command::Record => {
                let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                let recording = actions.toggle_recording().await?;
                Ok(ResponseData::Record(RecordData { recording }))
            }

            Command::Jukebox => {
                let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                let muted = actions.toggle_jukebox_muted().await?;
                let gain = actions.jukebox_gain().await;
                Ok(ResponseData::Jukebox(JukeboxData { muted, gain }))
            }

            Command::JukeboxVolume { level } => {
                let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                let gain = actions.set_jukebox_gain(f32::from(level) / 100.0).await?;
                let muted = actions.jukebox_muted().await;
                Ok(ResponseData::Jukebox(JukeboxData { muted, gain }))
            }

            Command::CreateGroup { name } => {
                let service = crate::groups::GroupService::new(app_handle.clone());
                Ok(ResponseData::Group(service.create(name).await?))
            }

            Command::JoinGroup { name } => {
                let service = crate::groups::GroupService::new(app_handle.clone());
                Ok(ResponseData::Group(service.join(name).await?))
            }

            Command::LeaveGroup => {
                let service = crate::groups::GroupService::new(app_handle.clone());
                Ok(ResponseData::Group(service.leave().await?))
            }

            Command::State => {
                let state_data = Self::query_state(app_handle).await;
                Ok(ResponseData::State(state_data))
            }

            Command::Targets => Ok(ResponseData::Targets(TargetsData {
                targets: Self::connect_targets(app_handle).await?,
            })),

            Command::Connect { id } => Self::connect_to(app_handle, &id).await,

            Command::Disconnect => {
                let stopped = crate::bedrock::BedrockConnector::new(app_handle.clone())
                    .disconnect()
                    .await?;
                Ok(ResponseData::Connect(ConnectData {
                    connected: false,
                    id: stopped.as_ref().map(|c| c.id.clone()),
                    name: stopped.as_ref().map(|c| c.name.clone()),
                }))
            }
        }
    }

    /// The worlds a controller may name.
    ///
    /// Errors rather than returning a partial list: both proxy and realm sessions need Xbox
    /// Live authentication, so a proxy-only list would name worlds that cannot be connected.
    async fn connect_targets(app_handle: &AppHandle) -> Result<Vec<ConnectTarget>, anyhow::Error> {
        Ok(crate::bedrock::BedrockTargetService::load_all(app_handle)
            .await?
            .targets())
    }

    async fn connect_to(app_handle: &AppHandle, id: &str) -> Result<ResponseData, anyhow::Error> {
        let service = crate::bedrock::BedrockTargetService::load_all(app_handle).await?;
        let target = service
            .resolve(id)
            .ok_or_else(|| anyhow::anyhow!("No target with id {}", id))?;

        let connector = crate::bedrock::BedrockConnector::new(app_handle.clone());
        match &target.address {
            crate::bedrock::ResolvedAddress::Proxy {
                host,
                port,
                protocol_version,
            } => {
                connector
                    .start_proxy(crate::bedrock::ProxyConnectRequest {
                        target_host: host.clone(),
                        target_port: *port,
                        listen_port: None,
                        network_interface: None,
                        advertised_protocol: *protocol_version,
                        // Left unresolved so the advertised list decides: a
                        // controller names a world, not how its addon reaches us.
                        addon_transport: None,
                    })
                    .await?;
            }
            crate::bedrock::ResolvedAddress::Realm { realm_id } => {
                connector
                    .start_realm(crate::bedrock::RealmConnectRequest {
                        realm_id: *realm_id,
                        realm_name: target.name.clone(),
                        network_interface: None,
                    })
                    .await?;
            }
        }

        log::info!("WebSocket connect: started {} ({})", target.name, target.id);
        Ok(ResponseData::Connect(ConnectData {
            connected: true,
            id: Some(target.id.clone()),
            name: Some(target.name.clone()),
        }))
    }

    /// The mode as the protocol names it. The client's own enum is a separate type, the
    /// same way `DeviceType` is.
    async fn voice_mode(app_handle: &AppHandle) -> VoiceMode {
        let mode = match app_handle.try_state::<Arc<crate::keybinds::KeybindListener>>() {
            Some(listener) => listener.voice_mode().await,
            None => KeybindVoiceMode::OpenMic,
        };
        match mode {
            KeybindVoiceMode::PushToTalk => VoiceMode::PushToTalk,
            KeybindVoiceMode::OpenMic => VoiceMode::OpenMic,
        }
    }

    /// Query current state from the app.
    async fn query_state(app_handle: &AppHandle) -> StateData {
        app_handle
            .state::<crate::audio::AudioActionsManager>()
            .query_state()
            .await
    }

    /// Build a full state JSON string for broadcasting
    async fn build_state_json(app_handle: &AppHandle) -> Result<String, serde_json::Error> {
        let state_data = Self::query_state(app_handle).await;
        let response = SuccessResponse::state(state_data);
        serde_json::to_string(&response)
    }
}
