use chrono::{DateTime, Utc};

pub enum TelemetryEvent {
    ServerStarted {
        at: DateTime<Utc>,
        hostname_sha: String,
    },
    Connected {
        at: DateTime<Utc>,
    },
    Disconnected {
        at: DateTime<Utc>,
        duration_secs: u64,
    },
    ChannelJoined {
        at: DateTime<Utc>,
    },
    ChannelLeft {
        at: DateTime<Utc>,
    },
}

impl TelemetryEvent {
    pub fn name(&self) -> &'static str {
        match self {
            TelemetryEvent::ServerStarted { .. } => "server_started",
            TelemetryEvent::Connected { .. } => "player_connected",
            TelemetryEvent::Disconnected { .. } => "player_disconnected",
            TelemetryEvent::ChannelJoined { .. } => "channel_joined",
            TelemetryEvent::ChannelLeft { .. } => "channel_left",
        }
    }

    pub fn at(&self) -> DateTime<Utc> {
        match self {
            TelemetryEvent::ServerStarted { at, .. } => *at,
            TelemetryEvent::Connected { at } => *at,
            TelemetryEvent::Disconnected { at, .. } => *at,
            TelemetryEvent::ChannelJoined { at } => *at,
            TelemetryEvent::ChannelLeft { at } => *at,
        }
    }
}
