use serde::{Deserialize, Serialize};

/// Who sent a packet, as the server determined it.
///
/// Server-authored only. A client never fills this in — the server takes the identity from
/// the mTLS certificate Common Name it authenticated at accept, and mints the device id from
/// the QUIC connection. Both are therefore facts the server established rather than a claim
/// the sender made about itself, which is what makes this safe to route and bill audio on.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct PacketSender {
    /// The canonical identity, `game:gamertag`, exactly as the certificate carried it.
    pub identity: String,
    /// Per-connection device id, so one player on two devices stays separately addressable.
    /// Minted by the server from the QUIC connection, which is what makes it unforgeable.
    ///
    /// `None` for something the server injected rather than a player who spoke — jukebox
    /// playback, webhook rides, channel API notifications. Absence rather than a reserved
    /// value, because s2n-quic hands out internal connection ids from a counter that starts
    /// at zero: the first connection after every restart holds id 0, so no numeric value is
    /// free to mean "not a connection".
    pub device: Option<u64>,
}

impl PacketSender {
    /// The channel API's identity on packets it injects, so a client can tell a membership
    /// change the server made from one a player made.
    pub const CHANNEL_API: &'static str = "channel_api";

    /// The server's own identity on packets it injects that belong to no API surface —
    /// position broadcasts and presence events.
    pub const SERVER_API: &'static str = "api";

    pub fn new(identity: String, device: u64) -> Self {
        Self {
            identity,
            device: Some(device),
        }
    }

    /// A sender for something the server injected rather than a player who spoke.
    pub fn synthetic(identity: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            device: None,
        }
    }

    pub fn is_synthetic(&self) -> bool {
        self.device.is_none()
    }
}
