use std::sync::Arc;

use common::s2n_quic::provider::event::{ConnectionInfo, ConnectionMeta, Subscriber, events};
use tokio::sync::watch;

use super::QuicLinkStats;

// Publishes transport measurements for whichever connection is currently live.
//
// The per-connection context *is* the stats handle, so a reconnect mints a fresh one and
// starts from zero rather than inheriting a dead connection's totals. Each new handle is
// announced on a watch channel, which is how a reader observes the current connection
// without holding a `Connection` and without a sentinel for "none yet".
#[derive(Debug)]
pub struct QuicStatsSubscriber {
    publisher: watch::Sender<Arc<QuicLinkStats>>,
}

impl QuicStatsSubscriber {
    pub fn new(publisher: watch::Sender<Arc<QuicLinkStats>>) -> Self {
        Self { publisher }
    }

    // Mints a handle for a new connection and announces it. Exposed rather than inlined into
    // `create_connection_context` so the fresh-handle-per-connection guarantee can be tested:
    // s2n-quic's `ConnectionMeta` is `#[non_exhaustive]`, so the trait method itself cannot be
    // called from a test.
    pub fn mint_context(&self) -> Arc<QuicLinkStats> {
        let stats = Arc::new(QuicLinkStats::new());
        let _ = self.publisher.send(stats.clone());
        stats
    }
}

impl Subscriber for QuicStatsSubscriber {
    type ConnectionContext = Arc<QuicLinkStats>;

    fn create_connection_context(
        &mut self,
        _meta: &ConnectionMeta,
        _info: &ConnectionInfo,
    ) -> Self::ConnectionContext {
        self.mint_context()
    }

    fn on_recovery_metrics(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::RecoveryMetrics,
    ) {
        context.record_rtt_for_path(
            event.path.is_active,
            event.smoothed_rtt,
            event.latest_rtt,
            event.min_rtt,
            event.rtt_variance,
        );
    }

    fn on_packet_sent(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        _event: &events::PacketSent,
    ) {
        context.record_sent();
    }

    fn on_packet_received(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::PacketReceived,
    ) {
        context.record_received();

        // Only the 1-RTT space. Initial and Handshake carry their own independent numbering, and
        // folding them into one sequence would read as enormous loss at each transition.
        if let events::PacketHeader::OneRtt { number, .. } = event.packet_header {
            context.record_packet_number(number);
        }
    }

    // Packets *this* endpoint sent that its own loss detection gave up on. This is uplink
    // loss; a packet the peer sent that never arrived is detected by the peer, not here.
    fn on_packet_lost(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        _event: &events::PacketLost,
    ) {
        context.record_lost();
    }

    fn on_path_created(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        _event: &events::PathCreated,
    ) {
        context.record_path();
    }

    // A generic drop count, deliberately not interpreted as a path-budget signal. When a
    // rotating source address exhausts the budget it is the *peer's* path manager that fills
    // and the peer that discards, so this stays zero through exactly that failure. The
    // client-side fingerprint is the send/receive stall the service derives instead.
    fn on_datagram_dropped(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        _event: &events::DatagramDropped,
    ) {
        context.record_datagram_dropped();
    }
}
