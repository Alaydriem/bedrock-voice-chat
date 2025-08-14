use common::structs::packet::QuicNetworkPacket;
use tokio::sync::mpsc;

/// Handles webhook HTTP requests and converts them to QUIC packets
#[derive(Clone)]
pub struct WebhookReceiver {
    webhook_tx: mpsc::UnboundedSender<QuicNetworkPacket>
}

impl WebhookReceiver {
    pub fn new(
        webhook_tx: mpsc::UnboundedSender<QuicNetworkPacket>
    ) -> Self {
        Self {
            webhook_tx
        }
    }

    /// Send a packet through the webhook system
    pub async fn send_packet(&self, packet: QuicNetworkPacket) -> Result<(), Box<dyn std::error::Error>> {
        self.webhook_tx.send(packet)?;
        Ok(())
    }
}
