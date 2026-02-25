use common::traits::StreamTrait;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use tokio::task::AbortHandle;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use serde::{Deserialize, Serialize};

pub mod structs;
pub use structs::{Command, CommandMessage, DeviceType, SuccessResponse, ErrorResponse, ResponseData, PongData, MuteData, RecordData, StateData};

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
            localhost_only: true,
            port: 9595,
            key: String::new(),
        }
    }
}

/// Wrapper around a broadcast sender for sharing with Tauri managed state.
/// UI commands (mute, recording) use this to push state updates to all connected WS clients.
pub struct WebSocketBroadcaster(pub broadcast::Sender<String>);

impl WebSocketBroadcaster {
    /// Serialize a StateData DTO and broadcast to all connected WS clients.
    pub fn broadcast_state(&self, state: StateData) {
        let response = SuccessResponse::state(state.muted, state.deafened, state.recording);
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = self.0.send(json);
        }
    }
}

pub struct WebSocketManager {
    abort_handle: Option<AbortHandle>,
    shutdown_tx: Option<watch::Sender<bool>>,
    config: Option<WebSocketConfig>,
    app_handle: AppHandle,
    broadcast_tx: broadcast::Sender<String>,
}

impl WebSocketManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let config: Option<WebSocketConfig> = app_handle.store("store.json")
            .ok()
            .and_then(|store| store.get("websocket_server"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let (broadcast_tx, _) = broadcast::channel(16);

        Self {
            abort_handle: None,
            shutdown_tx: None,
            config,
            app_handle,
            broadcast_tx,
        }
    }

    /// Extract a broadcaster handle for registration as Tauri managed state
    pub fn broadcaster(&self) -> WebSocketBroadcaster {
        WebSocketBroadcaster(self.broadcast_tx.clone())
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
        let config = self.config.as_ref()
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
    async fn start_server_loop(&self, shutdown_rx: watch::Receiver<bool>) -> Result<AbortHandle, anyhow::Error> {
        let config = self.config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config available"))?;

        let host = if config.localhost_only { "127.0.0.1" } else { "0.0.0.0" };
        let addr = format!("{}:{}", host, config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let config = config.clone();
        let app_handle = self.app_handle.clone();
        let broadcast_tx = self.broadcast_tx.clone();

        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app_handle = app_handle.clone();
                let key = config.key.clone();
                let broadcast_tx = broadcast_tx.clone();
                let shutdown_rx = shutdown_rx.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(stream, app_handle, key, broadcast_tx, shutdown_rx).await {
                        // Connection resets / broken pipes are normal client disconnects
                        let is_disconnect = e.root_cause()
                            .downcast_ref::<std::io::Error>()
                            .map_or(false, |io_err| matches!(io_err.kind(),
                                std::io::ErrorKind::ConnectionReset |
                                std::io::ErrorKind::ConnectionAborted |
                                std::io::ErrorKind::BrokenPipe
                            ));

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
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), anyhow::Error> {
        use tokio_tungstenite::accept_async;
        use futures_util::{StreamExt, SinkExt};

        let ws_stream = accept_async(stream).await?;
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

                    // Validate authentication key if one is configured
                    if !auth_key.is_empty() && parsed.key.as_deref() != Some(auth_key.as_str()) {
                        let error_response = ErrorResponse::new("Invalid authentication key".to_string());
                        let json = serde_json::to_string(&error_response)?;
                        write.send(tokio_tungstenite::tungstenite::Message::Text(json.into())).await?;
                        continue;
                    }

                    // Check if this is a state-changing command
                    let is_state_changing = matches!(parsed.command, Command::Mute { .. } | Command::Record);

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
        match cmd {
            Command::Ping => Ok(ResponseData::Pong(PongData { pong: true })),

            Command::Mute { device } => {
                // In PTT mode, input mute via WebSocket is a no-op
                if matches!(device, DeviceType::Input) {
                    if let Ok(store) = app_handle.store("store.json") {
                        if let Some(config) = store.get("keybinds") {
                            if let Some(mode) = config.get("voiceMode").and_then(|v| v.as_str()) {
                                if mode == "pushToTalk" {
                                    let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                                    let status = actions.is_muted(crate::audio::types::AudioDeviceType::InputDevice).await;
                                    return Ok(ResponseData::Mute(MuteData {
                                        device: "input".to_string(),
                                        muted: status,
                                    }));
                                }
                            }
                        }
                    }
                }

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

            Command::Record => {
                let actions = app_handle.state::<crate::audio::AudioActionsManager>();
                let recording = actions.toggle_recording().await?;
                Ok(ResponseData::Record(RecordData { recording }))
            }

            Command::State => {
                let state_data = Self::query_state(app_handle).await;
                Ok(ResponseData::State(state_data))
            }
        }
    }

    /// Query current muted/deafened/recording state from the app
    async fn query_state(app_handle: &AppHandle) -> StateData {
        let asm = app_handle.state::<tauri::async_runtime::Mutex<crate::AudioStreamManager>>();
        let mut asm = asm.lock().await;

        let muted = asm.mute_status(&crate::audio::types::AudioDeviceType::InputDevice).await.unwrap_or(false);
        let deafened = asm.mute_status(&crate::audio::types::AudioDeviceType::OutputDevice).await.unwrap_or(false);
        drop(asm);

        let recording_manager = app_handle.state::<Arc<tauri::async_runtime::Mutex<crate::audio::RecordingManager>>>();
        let manager = recording_manager.lock().await;
        let recording = manager.is_recording();

        StateData { muted, deafened, recording }
    }

    /// Build a full state JSON string for broadcasting
    async fn build_state_json(app_handle: &AppHandle) -> Result<String, serde_json::Error> {
        let state_data = Self::query_state(app_handle).await;
        let response = SuccessResponse::state(state_data.muted, state_data.deafened, state_data.recording);
        serde_json::to_string(&response)
    }
}
