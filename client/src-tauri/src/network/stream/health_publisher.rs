use common::structs::network::ConnectionHealth;
use tauri::AppHandle;

/// The single place a connection-health verdict is announced.
///
/// Every surface is fed from here rather than at each call site. Eight places produce a verdict,
/// and one that reached a single surface would leave a controller believing a link was up long
/// after it failed, with nothing on the wire to indicate why.
pub struct HealthPublisher;

impl HealthPublisher {
    pub fn publish(app_handle: &AppHandle, health: ConnectionHealth) {
        if let Some(broadcaster) =
            tauri::Manager::try_state::<crate::websocket::WebSocketBroadcaster>(app_handle)
        {
            broadcaster.broadcast_health(health);
        }
    }
}
