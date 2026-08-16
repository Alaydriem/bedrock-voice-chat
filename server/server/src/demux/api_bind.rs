use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use super::demux_error::DemuxError;
use super::loopback_port::LoopbackPort;

// Where the API listener lives, shared between the demultiplexer and Rocket.
//
// The port is picked by binding one and releasing it, so between that release and
// Rocket's own bind the number is free for anything else to take — rare on a host
// running one server, routine on a machine starting hundreds at once. Two things
// answer that:
//
// - The check happens immediately before Rocket binds rather than when the port
//   was first chosen, so the exposed gap is microseconds instead of the whole of
//   startup.
// - The port is shared rather than copied, so a port that has to be re-picked is
//   one the demultiplexer follows rather than one it has lost.
//
// Holding the listener across startup and releasing it at the last moment was
// tried and reverted. It does keep the port occupied for the whole window, but
// Rocket 0.5 binds from its own configuration — `http_server`, which would accept
// a listener already owned, is `pub(crate)` — so the handoff cannot be atomic
// anyway. Worse, a held listener answers a connect, because the kernel completes
// the handshake into the backlog whether or not anything accepts; the
// demultiplexer then had to wait for the hold to lift before probing, which
// lengthened startup enough to leave more test servers alive at once and cost
// more in contention than the race it closed.
#[derive(Clone)]
pub struct ApiBind {
    port: Arc<AtomicU16>,
}

impl ApiBind {
    pub fn new(port: u16) -> Self {
        Self {
            port: Arc::new(AtomicU16::new(port)),
        }
    }

    pub fn reserve() -> Result<Self, DemuxError> {
        Ok(Self::new(LoopbackPort::reserve()?))
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from((Ipv4Addr::LOCALHOST, self.port.load(Ordering::SeqCst)))
    }

    // Confirms the port can still be bound, picks another if it cannot, and
    // answers with the address to bind.
    //
    // Called immediately before Rocket binds and nowhere else. That placement is
    // the whole of its value: the shorter the gap between the check and the bind,
    // the smaller the window another process can take the port in.
    pub fn claim_for_bind(&self) -> Result<SocketAddr, DemuxError> {
        if TcpListener::bind(self.addr()).is_ok() {
            return Ok(self.addr());
        }

        let replacement = LoopbackPort::reserve()?;
        tracing::warn!(
            previous = self.port.load(Ordering::SeqCst),
            replacement,
            "the API loopback port was taken before it could be bound; using another"
        );
        self.port.store(replacement, Ordering::SeqCst);

        Ok(self.addr())
    }
}
