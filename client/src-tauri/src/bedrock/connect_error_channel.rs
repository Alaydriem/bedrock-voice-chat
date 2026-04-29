use std::sync::{Arc, OnceLock};

use common::bedrock_protocol::{Error, HandshakeFailureKind};
use common::structs::bedrock::BedrockConnectError;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 32;

static SENDER: OnceLock<Arc<broadcast::Sender<BedrockConnectError>>> = OnceLock::new();

pub struct BedrockConnectErrorChannel {
    sender: Arc<broadcast::Sender<BedrockConnectError>>,
}

impl BedrockConnectErrorChannel {
    pub fn init() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        let arc = Arc::new(tx);
        let _ = SENDER.set(arc.clone());
        Self { sender: arc }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<BedrockConnectError>> {
        Arc::clone(&self.sender)
    }
}

pub fn emit(error: BedrockConnectError) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(error);
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
