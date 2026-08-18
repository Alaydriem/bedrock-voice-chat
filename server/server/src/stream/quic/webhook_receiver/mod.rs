use common::structs::packet::QuicNetworkPacket;
use tokio::sync::mpsc;

// Where a packet entered this server.
//
// Carried alongside the packet rather than on it: `QuicNetworkPacket` is a wire
// type whose postcard encoding is positional, so a field there would be a breaking
// change in both directions to answer a question that never crosses the wire.
//
// The distinction is load-bearing. A packet that arrived from a peer must not be
// forwarded back out to peers, or the sender hears its own voice returned and any
// two peers sharing a world feed each other indefinitely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketOrigin {
    Local,
    Peer,
}

/// Handles webhook HTTP requests and converts them to QUIC packets
#[derive(Clone)]
pub struct WebhookReceiver {
    webhook_tx: mpsc::UnboundedSender<(QuicNetworkPacket, PacketOrigin)>,
}

impl WebhookReceiver {
    pub fn new(webhook_tx: mpsc::UnboundedSender<(QuicNetworkPacket, PacketOrigin)>) -> Self {
        Self { webhook_tx }
    }

    /// Send a packet through the webhook system
    pub async fn send_packet(
        &self,
        packet: QuicNetworkPacket,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.webhook_tx.send((packet, PacketOrigin::Local))?;
        Ok(())
    }
}

// A peer's admitted packets enter the same broadcast loop a local client's do, and
// are tagged so that loop does not forward them back out to peers. The plane never
// learns how local delivery works; this is the whole of the coupling between them.
impl crate::relay::PeerSink for WebhookReceiver {
    fn publish(&self, packet: QuicNetworkPacket) {
        if self.webhook_tx.send((packet, PacketOrigin::Peer)).is_err() {
            tracing::warn!("dropping a peer packet: the webhook loop has stopped");
        }
    }
}
