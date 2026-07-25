pub mod quic_close_code;

pub use quic_close_code::QuicCloseCode;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
}
