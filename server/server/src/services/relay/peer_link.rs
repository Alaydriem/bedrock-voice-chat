use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::peer_role::PeerRole;
use super::relayed_packet::RelayedPacket;

// A peer link is torn down after this much wall-clock time with no relayed
// traffic in either direction
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

// Bounded per-peer outbound queue. Forwarding uses `try_send` and drops on full
// so the audio hot path is never blocked by a slow/saturated peer.
const OUTBOUND_CAPACITY: usize = 1024;

// Whether this server dialed the peer (QUIC client) or accepted it (QUIC server
// — the side that issues the in-memory peer cert).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerDirection {
    Initiator,
    Acceptor,
}

// Lifecycle of a single peer endpoint. An inbound connection observed while
// `Dialing` adopts the inbound socket and cancels the pending dial (see
// `PeerLink::adopt_inbound`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerLinkState {
    Dialing,
    Connected,
    Idle,
    Closed,
}

// Owns one logical connection to a peer endpoint (`host:port`). Holds the
// role, lifecycle state, last-activity instant, and the bounded outbound queue
// that the forwarder drains onto the QUIC connection. The clock is injected via
// method parameters so idle logic is testable without real time.
pub struct PeerLink {
    endpoint: String,
    state: PeerLinkState,
    direction: PeerDirection,
    role: PeerRole,
    last_activity: Instant,
    outbound_tx: mpsc::Sender<RelayedPacket>,
    outbound_rx: Option<mpsc::Receiver<RelayedPacket>>,
}

impl PeerLink {
    pub fn new(endpoint: &str, direction: PeerDirection, now: Instant) -> Self {
        let (outbound_tx, outbound_rx) = mpsc::channel(OUTBOUND_CAPACITY);
        let state = match direction {
            PeerDirection::Initiator => PeerLinkState::Dialing,
            PeerDirection::Acceptor => PeerLinkState::Connected,
        };
        Self {
            endpoint: endpoint.to_string(),
            state,
            direction,
            role: PeerRole::Mesh,
            last_activity: now,
            outbound_tx,
            outbound_rx: Some(outbound_rx),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn state(&self) -> PeerLinkState {
        self.state
    }

    pub fn direction(&self) -> PeerDirection {
        self.direction
    }

    pub fn role(&self) -> PeerRole {
        self.role
    }

    pub fn set_role(&mut self, role: PeerRole) {
        self.role = role;
    }

    // Hands the receive half of the outbound queue to the peer-writer task. Can
    // be taken once; subsequent calls return `None`.
    pub fn take_outbound_receiver(&mut self) -> Option<mpsc::Receiver<RelayedPacket>> {
        self.outbound_rx.take()
    }

    // Cloneable sender used by the fan-out to enqueue outbound packets without
    // blocking the audio path.
    pub fn outbound_sender(&self) -> mpsc::Sender<RelayedPacket> {
        self.outbound_tx.clone()
    }

    pub fn mark_connected(&mut self, now: Instant) {
        self.state = PeerLinkState::Connected;
        self.last_activity = now;
    }

    // Resets the idle timer; called on every relayed packet in or out.
    pub fn mark_activity(&mut self, now: Instant) {
        if self.state == PeerLinkState::Idle {
            self.state = PeerLinkState::Connected;
        }
        self.last_activity = now;
    }

    pub fn is_idle(&self, now: Instant) -> bool {
        now.duration_since(self.last_activity) >= IDLE_TIMEOUT
    }

    pub fn close(&mut self) {
        self.state = PeerLinkState::Closed;
    }

    pub fn is_closed(&self) -> bool {
        self.state == PeerLinkState::Closed
    }

    // Adopts an inbound connection that arrived while this link was still
    // dialing: the pending dial is abandoned and the link becomes an acceptor.
    // Returns true if a dial was actually cancelled.
    pub fn adopt_inbound(&mut self, now: Instant) -> bool {
        let was_dialing = self.state == PeerLinkState::Dialing;
        self.direction = PeerDirection::Acceptor;
        self.state = PeerLinkState::Connected;
        self.last_activity = now;
        was_dialing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn initiator_starts_dialing_acceptor_starts_connected() {
        let now = Instant::now();
        let i = PeerLink::new("a:1", PeerDirection::Initiator, now);
        let a = PeerLink::new("b:1", PeerDirection::Acceptor, now);
        assert_eq!(i.state(), PeerLinkState::Dialing);
        assert_eq!(a.state(), PeerLinkState::Connected);
    }

    #[test]
    fn idle_after_timeout_resets_on_activity() {
        let t0 = Instant::now();
        let mut link = PeerLink::new("a:1", PeerDirection::Acceptor, t0);
        link.mark_activity(t0);
        assert!(!link.is_idle(t0 + secs(299)));
        assert!(link.is_idle(t0 + secs(301)));
        link.mark_activity(t0 + secs(301));
        assert!(!link.is_idle(t0 + secs(360)));
    }

    #[test]
    fn inbound_while_dialing_adopts_and_cancels_dial() {
        let t0 = Instant::now();
        let mut link = PeerLink::new("a:1", PeerDirection::Initiator, t0);
        assert_eq!(link.state(), PeerLinkState::Dialing);
        let cancelled = link.adopt_inbound(t0 + secs(1));
        assert!(cancelled, "a pending dial should be cancelled");
        assert_eq!(link.direction(), PeerDirection::Acceptor);
        assert_eq!(link.state(), PeerLinkState::Connected);
    }

    #[test]
    fn adopt_inbound_when_not_dialing_reports_no_cancel() {
        let t0 = Instant::now();
        let mut link = PeerLink::new("a:1", PeerDirection::Acceptor, t0);
        let cancelled = link.adopt_inbound(t0 + secs(1));
        assert!(!cancelled);
    }

    #[test]
    fn role_defaults_to_mesh_and_is_settable() {
        let mut link = PeerLink::new("a:1", PeerDirection::Acceptor, Instant::now());
        assert_eq!(link.role(), PeerRole::Mesh);
        link.set_role(PeerRole::Hub);
        assert_eq!(link.role(), PeerRole::Hub);
    }
}
