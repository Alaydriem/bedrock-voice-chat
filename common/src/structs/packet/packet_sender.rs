use serde::{Deserialize, Serialize};

/// Who sent a packet, as the server determined it.
///
/// Server-authored only. A client never fills this in — the server takes the identity from
/// the mTLS certificate Common Name it authenticated at accept, and mints the device id from
/// the QUIC connection. Both are therefore facts the server established rather than a claim
/// the sender made about itself, which is what makes this safe to route and bill audio on.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketSender {
    /// A player. `device` is `Some` for a connection this server holds and `None` for a
    /// player reached over the relay, who is a real human this server has no socket to.
    Player {
        identity: crate::PlayerIdentity,
        /// Per-connection device id, so one player on two devices stays separately
        /// addressable. Minted by the server from the QUIC connection, which is what makes
        /// it unforgeable.
        device: Option<u64>,
    },
    /// A connection this server holds, with the identity left out. The receiver resolves it
    /// from the device id it has already been told.
    ///
    /// Produced only by the audio fan-out, on frames between position heartbeats. Never
    /// inbound.
    Device(u64),
    /// The server itself rather than a player: the channel API, position broadcasts,
    /// jukebox playback. Not a canonical identity, which is why it is not a
    /// `PlayerIdentity`.
    Service(String),
}

impl PacketSender {
    /// The channel API's name on packets it injects, so a client can tell a membership
    /// change the server made from one a player made.
    pub const CHANNEL_API: &'static str = "channel_api";

    /// The server's own name on packets it injects that belong to no API surface —
    /// position broadcasts and presence events.
    pub const SERVER_API: &'static str = "api";

    pub fn player(identity: crate::PlayerIdentity, device: u64) -> Self {
        Self::Player {
            identity,
            device: Some(device),
        }
    }

    /// A player this server holds no connection for, reached over the relay.
    pub fn relayed(identity: crate::PlayerIdentity) -> Self {
        Self::Player {
            identity,
            device: None,
        }
    }

    /// A sender for something the server injected rather than a player who spoke.
    ///
    /// Named `for_service` rather than `service` because the accessor below already owns
    /// that name, and two associated items cannot share one.
    pub fn for_service(name: impl Into<String>) -> Self {
        Self::Service(name.into())
    }

    /// The player this came from, when the packet names one.
    ///
    /// `None` for `Device`, where the identity was elided and must be resolved from the
    /// device id, and for `Service`, which is not a player at all. The two are distinct and
    /// a caller that treats them alike will attribute server audio to a player.
    pub fn identity(&self) -> Option<&crate::PlayerIdentity> {
        match self {
            Self::Player { identity, .. } => Some(identity),
            Self::Device(_) | Self::Service(_) => None,
        }
    }

    /// The key this sender is routed and keyed on.
    ///
    /// A player's canonical identity rendered, or the service name for server-injected
    /// audio. Both populate the position cache and the channel map, so routing needs one
    /// key that covers either — which is what `identity` deliberately does not give it.
    ///
    /// `None` for `Device`, which carries no key and is never inbound.
    pub fn routing_key(&self) -> Option<String> {
        match self {
            Self::Player { identity, .. } => Some(identity.to_string()),
            Self::Service(name) => Some(name.clone()),
            Self::Device(_) => None,
        }
    }

    pub fn device(&self) -> Option<u64> {
        match self {
            Self::Player { device, .. } => *device,
            Self::Device(device) => Some(*device),
            Self::Service(_) => None,
        }
    }

    pub fn service(&self) -> Option<&str> {
        match self {
            Self::Service(name) => Some(name.as_str()),
            Self::Player { .. } | Self::Device(_) => None,
        }
    }
}
