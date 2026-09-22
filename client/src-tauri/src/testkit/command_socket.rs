use crate::websocket::{WebSocketConfig, WebSocketManager};
use common::traits::StreamTrait;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager};

/// Binds the operator-facing command WebSocket on demand, so a scenario can speak the
/// protocol a Stream Deck plugin speaks.
///
/// The app declines to bind it at start-up until a token exists, and the e2e bin has no
/// settings pane to mint one. This supplies the token and the port instead.
pub struct CommandSocket;

impl CommandSocket {
    /// Bind the listener and report the port it reached.
    ///
    /// `port` is a preference rather than a promise: `ListenerBinder` moves off a port
    /// already in use, and a caller that assumed otherwise would dial nothing.
    pub async fn start(
        app_handle: &AppHandle,
        port: u16,
        key: String,
    ) -> Result<u16, anyhow::Error> {
        let manager = app_handle.state::<Mutex<WebSocketManager>>();
        let mut manager = manager.lock().await;

        manager.update_config(WebSocketConfig {
            key,
            port,
            ..WebSocketConfig::default()
        });

        // A listener already up is left alone and its port reported: `start` refuses a second
        // call, and that refusal is not a failure the caller has to distinguish.
        if manager.external_port().is_none() {
            manager.start().await?;
        }

        manager
            .external_port()
            .ok_or_else(|| anyhow::anyhow!("command listener bound but reported no port"))
    }
}
