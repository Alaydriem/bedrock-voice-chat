use super::DemuxError;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};

/// Reserves a loopback port for a backend that cannot be handed a listener.
///
/// Rocket 0.5 binds its own socket and reports no address back, so its port has to be
/// known before it is configured. Asking the host for an ephemeral one and releasing it
/// is the only way to learn a free port here; Rocket then binds it moments later during
/// the same startup sequence.
///
/// Anything that *can* keep its listener should keep it and read `local_addr()` instead.
pub struct LoopbackPort;

impl LoopbackPort {
    // A handful of attempts covers the case where something else took the port between
    // the release and the caller's bind. Exhausting them means the host is not handing
    // out usable ephemeral ports, which is a startup failure rather than a retry loop.
    const ATTEMPTS: usize = 8;

    pub fn reserve() -> Result<u16, DemuxError> {
        let mut last_error = None;

        for _ in 0..Self::ATTEMPTS {
            match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))) {
                Ok(listener) => match listener.local_addr() {
                    Ok(addr) => return Ok(addr.port()),
                    Err(e) => last_error = Some(e),
                },
                Err(e) => last_error = Some(e),
            }
        }

        Err(DemuxError::PortReservation {
            attempts: Self::ATTEMPTS,
            source: last_error.unwrap_or_else(|| {
                std::io::Error::other("no error reported by the operating system")
            }),
        })
    }
}
