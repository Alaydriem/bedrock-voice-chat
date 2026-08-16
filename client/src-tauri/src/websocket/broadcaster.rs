use common::structs::network::ConnectionHealth;
use tokio::sync::{broadcast, watch};

use super::{StateData, SuccessResponse};

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
    pub(super) fn new(
        commands: broadcast::Sender<String>,
        metrics: broadcast::Sender<String>,
        health: watch::Sender<ConnectionHealth>,
    ) -> Self {
        Self {
            commands,
            metrics,
            health,
        }
    }

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
