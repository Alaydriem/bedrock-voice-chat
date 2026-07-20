use serde::{Deserialize, Serialize};

/// Per-bot outcome for one run. `frames_received` is the bot's `frames_from_quic`
/// (AudioFrame datagrams it got from other players via the server); comparing it
/// to co-group senders' `frames_sent` is the delivery-loss signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotReport {
    pub name: String,
    pub connected: bool,
    pub disconnected_early: bool,
    pub frames_sent: u64,
    pub frames_received: u64,
}

/// One host's aggregated result, printed as a single JSON line on the agent's
/// stdout and collected by the controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    pub host: String,
    pub connected: usize,
    pub total_frames_sent: u64,
    pub total_frames_received: u64,
    pub bots: Vec<BotReport>,
}

impl AgentReport {
    pub fn from_bots(host: String, bots: Vec<BotReport>) -> Self {
        let connected = bots.iter().filter(|b| b.connected).count();
        let total_frames_sent = bots.iter().map(|b| b.frames_sent).sum();
        let total_frames_received = bots.iter().map(|b| b.frames_received).sum();
        Self {
            host,
            connected,
            total_frames_sent,
            total_frames_received,
            bots,
        }
    }
}
