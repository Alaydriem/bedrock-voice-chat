use std::sync::Arc;

use common::bedrock_protocol::{Error, HandshakeFailureKind};
use common::structs::bedrock::BedrockConnectError;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 32;

pub struct BedrockConnectErrorChannel {
    sender: Arc<broadcast::Sender<BedrockConnectError>>,
}

impl BedrockConnectErrorChannel {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender: Arc::new(tx),
        }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<BedrockConnectError>> {
        Arc::clone(&self.sender)
    }

    pub fn emit(&self, error: BedrockConnectError) {
        let _ = self.sender.send(error);
    }
}

impl Default for BedrockConnectErrorChannel {
    fn default() -> Self {
        Self::new()
    }
}

pub fn classify(error: &Error) -> BedrockConnectError {
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
        Error::RakNet(msg) | Error::Nethernet(msg) | Error::Signaling(msg) | Error::WebRtc(msg) => {
            BedrockConnectError::Transport {
                message: msg.clone(),
            }
        }
        other => BedrockConnectError::Other {
            message: other.to_string(),
        },
    }
}
