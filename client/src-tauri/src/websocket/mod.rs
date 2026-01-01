use common::traits::StreamTrait;
use std::sync::Arc;
use tokio::task::AbortHandle;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::{Store, StoreExt};
use serde::{Deserialize, Serialize};

pub mod structs;
pub use structs::{Command, DeviceType, SuccessResponse, ErrorResponse, ResponseData, PongData, MuteData, RecordData};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub key: String,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 9595,
            key: String::new(),
        }
    }
}

pub struct WebSocketManager {
    abort_handle: Option<AbortHandle>,
    config: Option<WebSocketConfig>,
    app_handle: AppHandle,
}

impl WebSocketManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let config: Option<WebSocketConfig> = app_handle.store("store.json")
            .ok()
            .and_then(|store| store.get("websocket_server"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        Self {
            abort_handle: None,
            config,
            app_handle,
        }
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

        if config.key.is_empty() {
            return Err(anyhow::anyhow!("Encryption key is required"));
        }

        let handle = self.start_server_loop().await?;
        self.abort_handle = Some(handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
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
    async fn start_server_loop(&self) -> Result<AbortHandle, anyhow::Error> {
        let config = self.config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config available"))?;

        let addr = format!("{}:{}", config.host, config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let config = config.clone();
        let app_handle = self.app_handle.clone();

        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app_handle = app_handle.clone();
                let key = config.key.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(stream, app_handle, key).await {
                        log::error!("Connection error: {}", e);
                    }
                });
            }
        });

        Ok(handle.abort_handle())
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        app_handle: AppHandle,
        _auth_key: String,
    ) -> Result<(), anyhow::Error> {
        use tokio_tungstenite::accept_async;
        use futures_util::{StreamExt, SinkExt};

        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg?;

            if msg.is_text() || msg.is_binary() {
                let text = msg.to_text()?;

                // Parse command and execute
                let response_json = match Self::execute_command(text, &app_handle).await {
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
            }
        }

        Ok(())
    }

    async fn execute_command(
        text: &str,
        app_handle: &AppHandle,
    ) -> Result<ResponseData, anyhow::Error> {
        let cmd = Command::from_json(text)?;

        match cmd {
            Command::Ping => Ok(ResponseData::Pong(PongData { pong: true })),

            Command::Mute { device } => {
                let audio_device = match device {
                    DeviceType::Input => crate::audio::types::AudioDeviceType::InputDevice,
                    DeviceType::Output => crate::audio::types::AudioDeviceType::OutputDevice,
                };

                let asm = app_handle.state::<tauri::async_runtime::Mutex<crate::AudioStreamManager>>();
                let mut asm = asm.lock().await;
                asm.toggle(&audio_device, common::structs::audio::StreamEvent::Mute).await?;

                let status = asm.mute_status(&audio_device).await.unwrap_or(false);
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
                let recording_manager = app_handle.state::<Arc<tauri::async_runtime::Mutex<crate::audio::RecordingManager>>>();
                let mut manager = recording_manager.lock().await;

                if manager.is_recording() {
                    manager.stop_recording().await?;
                    Ok(ResponseData::Record(RecordData { recording: false }))
                } else {
                    // Get current player from store
                    let current_player = app_handle.store("store.json")?
                        .get("current_player")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .ok_or_else(|| anyhow::anyhow!("No current player"))?;

                    manager.start_recording(current_player).await?;
                    Ok(ResponseData::Record(RecordData { recording: true }))
                }
            }
        }
    }
}
