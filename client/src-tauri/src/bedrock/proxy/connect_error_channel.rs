use std::sync::Arc;

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
