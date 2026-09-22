use common::structs::audio::LevelSnapshot;
use common::structs::network::ConnectionHealth;
use common::structs::voice::VoiceRoster;
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
    // Everything this client pushes, on one channel. A `/events` subscriber reads the metrics
    // and health frames too, so they are sent here as well as to `metrics`.
    pub events: broadcast::Sender<String>,
    // Retained for the same reason as `health`: a window that reconnects between changes has
    // nothing to wait for, because the level publisher never re-sends silence.
    levels: watch::Sender<LevelSnapshot>,
    // Retained for the same reason again. Membership changes when somebody joins a group or
    // walks into earshot, which in a settled session is not for hours, so a subscriber that
    // arrives between changes would otherwise have nothing to draw.
    roster: watch::Sender<VoiceRoster>,
}

impl WebSocketBroadcaster {
    pub(super) fn new(
        commands: broadcast::Sender<String>,
        metrics: broadcast::Sender<String>,
        health: watch::Sender<ConnectionHealth>,
        events: broadcast::Sender<String>,
        levels: watch::Sender<LevelSnapshot>,
        roster: watch::Sender<VoiceRoster>,
    ) -> Self {
        Self {
            commands,
            metrics,
            health,
            events,
            levels,
            roster,
        }
    }

    /// A broadcaster wired to nothing, for exercising what it retains.
    ///
    /// The channels are real; there is simply nothing subscribed to them, which is the normal
    /// state of this object before any client connects.
    pub fn for_test() -> Self {
        let (commands, _) = broadcast::channel(16);
        let (metrics, _) = broadcast::channel(16);
        let (events, _) = broadcast::channel(16);
        let (health, _) = watch::channel(ConnectionHealth::Disconnected);
        let (levels, _) = watch::channel(LevelSnapshot::silent());
        let (roster, _) = watch::channel(VoiceRoster::empty(String::new()));
        Self::new(commands, metrics, health, events, levels, roster)
    }

    /// What a subscriber that has just arrived is told, before anything is forwarded.
    pub fn seed_frames(&self) -> Vec<String> {
        let mut frames = Vec::new();
        let health = common::structs::push::HealthPush::new(self.latest_health());
        if let Ok(json) = serde_json::to_string(&health) {
            frames.push(json);
        }
        let levels = common::structs::push::LevelsPush::new(self.levels.borrow().clone());
        if let Ok(json) = serde_json::to_string(&levels) {
            frames.push(json);
        }
        let roster = common::structs::push::RosterPush::new(self.latest_roster());
        if let Ok(json) = serde_json::to_string(&roster) {
            frames.push(json);
        }
        frames
    }

    /// Publish the roster to `/events` subscribers, and retain it for whoever subscribes next.
    pub fn broadcast_roster(&self, roster: VoiceRoster) {
        self.roster.send_replace(roster.clone());
        let push = common::structs::push::RosterPush::new(roster);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.events.send(json);
        }
    }

    /// The last roster published, for a subscriber that has just arrived.
    pub fn latest_roster(&self) -> VoiceRoster {
        self.roster.borrow().clone()
    }

    /// Serialize a diagnostics snapshot and broadcast it to `/metrics` subscribers.
    ///
    /// The envelope is tagged. `ResponseData` is `#[serde(untagged)]`, so a consumer could not
    /// distinguish a metrics frame from a state frame by shape alone if this rode on that enum.
    pub fn broadcast_metrics(&self, snapshot: common::structs::metrics::LinkDiagnosticsSnapshot) {
        let push = common::structs::push::MetricsPush::new(snapshot);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.metrics.send(json.clone());
            let _ = self.events.send(json);
        }
    }

    /// Broadcast a connection-health verdict to `/metrics` subscribers, and retain it for
    /// whoever subscribes next.
    ///
    /// Rides the metrics channel rather than the command channel: it describes the link a metrics
    /// subscriber is measuring, and a command client has `state` for what it cares about.
    pub fn broadcast_health(&self, health: ConnectionHealth) {
        let push = common::structs::push::HealthPush::new(health.clone());
        // `send_replace` rather than `send`: nothing holds a receiver for this channel — it is
        // read with `borrow` at seed time — and `send` refuses to store a value when the
        // receiver count is zero, which made the retention this channel exists for inert.
        self.health.send_replace(health);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.metrics.send(json.clone());
            let _ = self.events.send(json);
        }
    }

    /// Publish one level snapshot to `/events` subscribers, and retain it for the next arrival.
    pub fn broadcast_levels(&self, snapshot: LevelSnapshot) {
        self.levels.send_replace(snapshot.clone());
        let push = common::structs::push::LevelsPush::new(snapshot);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.events.send(json);
        }
    }

    /// Publish one unquantised capture level.
    ///
    /// Not retained. It is only produced while something is metering, and a stale amplitude
    /// handed to a calibration screen reads as a live microphone.
    pub fn broadcast_input_level(&self, level: common::structs::audio::InputLevel) {
        let push = common::structs::push::InputLevelPush::new(level);
        if let Ok(json) = serde_json::to_string(&push) {
            let _ = self.events.send(json);
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
