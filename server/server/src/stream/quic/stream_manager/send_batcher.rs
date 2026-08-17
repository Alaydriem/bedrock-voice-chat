use crate::stream::quic::connection::RoutedPacket;
use bytes::Bytes;
use std::time::Duration;
use tokio::sync::mpsc;

// Collects one flush's worth of outbound datagrams from a session's routed queue.
//
// The per-datagram cost of this path is dominated by the kernel: every flush is a
// connection wakeup and at least one UDP send whatever it carries. Waiting briefly
// after the first datagram lets frames other speakers routed in the same window ride
// the same flush, so the transport packs them into fewer packets.
pub struct SendBatcher {
    wait: Duration,
}

impl SendBatcher {
    // Bounds one flush. Voice rates never accumulate this much inside a wait window;
    // the bound exists so a burst cannot grow a flush without limit.
    const MAX_BATCH: usize = 32;

    pub fn new(wait_micros: u64) -> Self {
        Self {
            wait: Duration::from_micros(wait_micros),
        }
    }

    // One batch: blocks for the first datagram, then drains whatever arrived within
    // `wait`. `None` means the channel closed, which ends the session's send loop.
    pub async fn collect(
        &self,
        rx: &mut mpsc::Receiver<RoutedPacket>,
        out: &mut Vec<Bytes>,
    ) -> Option<()> {
        out.clear();
        let RoutedPacket::Serialized(first) = rx.recv().await?;
        out.push(first);

        if !self.wait.is_zero() {
            tokio::time::sleep(self.wait).await;
        }

        while out.len() < Self::MAX_BATCH {
            match rx.try_recv() {
                Ok(RoutedPacket::Serialized(bytes)) => out.push(bytes),
                Err(_) => break,
            }
        }
        Some(())
    }
}
