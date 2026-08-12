use common::structs::network::ConnectionHealth;
use tauri::{AppHandle, Emitter};

/// The single place a connection-health verdict is announced.
///
/// Both surfaces are fed from here rather than at each call site. Eight places produce a verdict,
/// and one that emitted to the window without broadcasting would leave a controller believing a
/// link was up long after it failed, with nothing on the wire to indicate why.
pub struct HealthPublisher;

impl HealthPublisher {
    pub const EVENT: &'static str = "connection_health";

    pub fn publish(app_handle: &AppHandle, health: ConnectionHealth) {
        if let Some(broadcaster) =
            tauri::Manager::try_state::<crate::websocket::WebSocketBroadcaster>(app_handle)
        {
            broadcaster.broadcast_health(health.clone());
        }

        let _ = app_handle.emit(Self::EVENT, health);
    }
}
