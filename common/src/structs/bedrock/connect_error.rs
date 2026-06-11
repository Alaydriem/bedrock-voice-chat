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

#[cfg(feature = "bedrock-protocol")]
impl From<&crate::bedrock_protocol::Error> for BedrockConnectError {
    fn from(error: &crate::bedrock_protocol::Error) -> Self {
        use crate::bedrock_protocol::{Error, HandshakeFailureKind};
        match error {
            Error::HandshakeFailed(kind) => match kind {
                HandshakeFailureKind::NethernetRejectedNoFallback {
                    bds_reason_code,
                    bds_kick_message,
                } => BedrockConnectError::NethernetRejectedNoFallback {
                    bds_reason_code: *bds_reason_code,
                    bds_kick_message: bds_kick_message.clone(),
                },
                HandshakeFailureKind::BdsRejectedOriginalLogin { kick_message } => {
                    BedrockConnectError::BdsRejectedOriginalLogin {
                        kick_message: kick_message.clone(),
                    }
                }
                HandshakeFailureKind::BdsRejectedOriginalLoginUndecoded => {
                    BedrockConnectError::BdsRejectedOriginalLoginUndecoded
                }
                HandshakeFailureKind::Other(msg) => BedrockConnectError::HandshakeOther {
                    message: msg.clone(),
                },
            },
            Error::Auth(msg) => BedrockConnectError::Auth {
                message: msg.clone(),
            },
            Error::RakNet(msg)
            | Error::Nethernet(msg)
            | Error::Signaling(msg)
            | Error::WebRtc(msg) => BedrockConnectError::Transport {
                message: msg.clone(),
            },
            other => BedrockConnectError::Other {
                message: other.to_string(),
            },
        }
    }
}
