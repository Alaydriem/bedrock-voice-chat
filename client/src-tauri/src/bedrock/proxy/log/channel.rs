use std::sync::Arc;

use common::structs::bedrock::BedrockLogEntry;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 512;

pub struct BedrockLogChannel {
    sender: Arc<broadcast::Sender<BedrockLogEntry>>,
}

impl BedrockLogChannel {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender: Arc::new(tx),
        }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<BedrockLogEntry>> {
        Arc::clone(&self.sender)
    }
}

impl Default for BedrockLogChannel {
    fn default() -> Self {
        Self::new()
    }
}
