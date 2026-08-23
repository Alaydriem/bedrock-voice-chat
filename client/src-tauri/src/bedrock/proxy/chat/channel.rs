use std::sync::Arc;

use tokio::sync::broadcast;

use super::line::ChatLine;

// Chat is bursty and disposable. A subscriber that falls behind should lose the oldest lines
// rather than stall the session loop, which broadcast gives for free via Lagged.
const CHANNEL_CAPACITY: usize = 256;

/// Carries decoded realm chat out of the proxy session loop.
///
/// A channel rather than an `AppHandle` on purpose: an `AppHandle` field drags the Tauri GUI
/// into test binaries through drop glue, so the emit happens outside the proxy.
pub struct BedrockChatChannel {
    sender: Arc<broadcast::Sender<ChatLine>>,
}

impl BedrockChatChannel {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender: Arc::new(tx),
        }
    }

    pub fn sender(&self) -> Arc<broadcast::Sender<ChatLine>> {
        Arc::clone(&self.sender)
    }

    pub fn emit(&self, line: ChatLine) {
        // Fails only when nothing is subscribed, which is the ordinary state before the
        // dashboard mounts. Chat that nobody is listening for is not worth logging about.
        let _ = self.sender.send(line);
    }
}

impl Default for BedrockChatChannel {
    fn default() -> Self {
        Self::new()
    }
}
