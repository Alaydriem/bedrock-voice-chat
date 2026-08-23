pub mod quic_close_code;
pub mod quic_port_selection;
pub mod voice_protocol;

pub use quic_close_code::QuicCloseCode;
pub use quic_port_selection::QuicPortSelection;
pub use voice_protocol::VoiceProtocol;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "status")]
pub enum ConnectionHealth {
    Connected,
    Reconnecting {
        attempt: u32,
    },
    Disconnected,
    Failed,
    VersionMismatch {
        client_version: String,
        server_version: String,
        client_too_old: bool,
    },
    // The server refused this connection's identity. Terminal: the client stops
    // reconnecting, because retrying with the same credentials cannot succeed.
    Unauthorized {
        reason: String,
    },
    // The server holds as many voice sessions as its operator permits. NOT terminal: the
    // same client succeeds once a slot frees, so the reconnect backoff continues. A client
    // that retried a full server without backing off would be a denial of service against
    // the server it wants to join.
    AtCapacity {
        limit: u32,
    },
}
