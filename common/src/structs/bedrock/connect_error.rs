use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BedrockConnectError {
    NethernetRejectedNoFallback {
        bds_reason_code: Option<u32>,
        bds_kick_message: Option<String>,
    },
    BdsRejectedOriginalLogin {
        kick_message: String,
    },
    BdsRejectedOriginalLoginUndecoded,
    HandshakeOther {
        message: String,
    },
    Auth {
        message: String,
    },
    Transport {
        message: String,
    },
    Other {
        message: String,
    },
}
