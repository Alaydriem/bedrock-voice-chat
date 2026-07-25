use serde::{Deserialize, Serialize};
use ts_rs::TS;

// QUIC application error codes this server closes connections with. A QUIC
// CONNECTION_CLOSE carries only this number — there is no reason phrase — so the
// numeric value is the protocol contract between server and client and must stay
// stable across versions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum QuicCloseCode {
    // The connection presented no usable mTLS identity, or one that is not a valid
    // issued certificate CN. The client must not retry: re-dialing cannot produce a
    // different outcome without new credentials.
    Unauthorized,
}

impl QuicCloseCode {
    pub fn as_u64(&self) -> u64 {
        match self {
            QuicCloseCode::Unauthorized => 4001,
        }
    }

    pub fn from_u64(value: u64) -> Option<QuicCloseCode> {
        match value {
            4001 => Some(QuicCloseCode::Unauthorized),
            _ => None,
        }
    }
}
