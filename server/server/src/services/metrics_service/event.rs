use chrono::{DateTime, Utc};

use crate::services::metrics_service::heartbeat_snapshot::HeartbeatSnapshot;
use crate::services::metrics_service::host_capability::HostCapability;

pub enum TelemetryEvent {
    ServerStarted {
        at: DateTime<Utc>,
    },
    Heartbeat {
        at: DateTime<Utc>,
        snapshot: HeartbeatSnapshot,
    },
    PlayerConnected {
        at: DateTime<Utc>,
    },
    PlayerDisconnected {
        at: DateTime<Utc>,
        duration_secs: u64,
    },
    ChannelJoined {
        at: DateTime<Utc>,
    },
    ChannelLeft {
        at: DateTime<Utc>,
    },
    Stopped {
        at: DateTime<Utc>,
        uptime_secs: u64,
        stop_reason: &'static str,
    },
    FirstSeen {
        at: DateTime<Utc>,
    },
    PlayerReconnected {
        at: DateTime<Utc>,
        time_since_disconnect_secs: u64,
    },
    // Reported by a Java mod, which has no telemetry channel of its own. Named for
    // the mod rather than the server because the fact it carries is about the
    // Minecraft host, not about this server.
    ModHostCapability {
        at: DateTime<Utc>,
        report: HostCapability,
    },
}

impl TelemetryEvent {
    pub fn name(&self) -> &'static str {
        match self {
            TelemetryEvent::ServerStarted { .. } => "Server::Started",
            TelemetryEvent::Heartbeat { .. } => "Server::Heartbeat",
            TelemetryEvent::PlayerConnected { .. } => "Server::PlayerConnected",
            TelemetryEvent::PlayerDisconnected { .. } => "Server::PlayerDisconnected",
            TelemetryEvent::ChannelJoined { .. } => "Server::ChannelJoined",
            TelemetryEvent::ChannelLeft { .. } => "Server::ChannelLeft",
            TelemetryEvent::Stopped { .. } => "Server::Stopped",
            TelemetryEvent::FirstSeen { .. } => "Server::FirstSeen",
            TelemetryEvent::PlayerReconnected { .. } => "Server::PlayerReconnected",
            TelemetryEvent::ModHostCapability { .. } => "Mod::HostCapability",
        }
    }

    pub fn at(&self) -> DateTime<Utc> {
        match self {
            TelemetryEvent::ServerStarted { at } => *at,
            TelemetryEvent::Heartbeat { at, .. } => *at,
            TelemetryEvent::PlayerConnected { at } => *at,
            TelemetryEvent::PlayerDisconnected { at, .. } => *at,
            TelemetryEvent::ChannelJoined { at } => *at,
            TelemetryEvent::ChannelLeft { at } => *at,
            TelemetryEvent::Stopped { at, .. } => *at,
            TelemetryEvent::FirstSeen { at } => *at,
            TelemetryEvent::PlayerReconnected { at, .. } => *at,
            TelemetryEvent::ModHostCapability { at, .. } => *at,
        }
    }
}
