use common::structs::packet::QuicNetworkPacket;
use tokio::sync::mpsc;

/// Handles webhook HTTP requests and converts them to QUIC packets
#[derive(Clone)]
pub struct WebhookReceiver {
    webhook_tx: mpsc::UnboundedSender<QuicNetworkPacket>,
}

impl WebhookReceiver {
    pub fn new(webhook_tx: mpsc::UnboundedSender<QuicNetworkPacket>) -> Self {
        Self { webhook_tx }
    }

    /// Send a packet through the webhook system
    pub async fn send_packet(
        &self,
        packet: QuicNetworkPacket,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.webhook_tx.send(packet)?;
        Ok(())
    }
}

// A peer's admitted packets enter the same broadcast loop a local client's do.
// The plane never learns how local delivery works; this is the whole of the
// coupling between them.
impl crate::relay::PeerSink for WebhookReceiver {
    fn publish(&self, packet: QuicNetworkPacket) {
        if self.webhook_tx.send(packet).is_err() {
            tracing::warn!("dropping a peer packet: the webhook loop has stopped");
        }
    }
}
