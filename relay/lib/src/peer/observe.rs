use std::net::SocketAddr;
use std::time::Duration;

use common::structs::relay::wire::Framing;
use iroh::endpoint::{Connection, IncomingAddr};
use iroh::{Endpoint, EndpointAddr};

use super::error::PeerError;

// Tells a node the address it was seen at.
//
// A node behind NAT observes only its own LAN address, so a peer ticket minted there
// names somewhere nobody can dial. This is the one thing the registry does for
// peering, and it is deliberately the whole of it: an address is returned and the
// connection closes. Nothing is relayed, and nothing is remembered.
//
// One exchange in one direction, so there is no in-band version negotiation. The ALPN
// carries the version and a second revision would be `bvc-observe/2`.
pub struct AddressObserver;

impl AddressObserver {
    pub const ALPN: &'static [u8] = b"bvc-observe/1";

    // Long enough to cross the internet twice, short enough that an operator waiting
    // on a peer link command does not think it hung.
    pub const TIMEOUT: Duration = Duration::from_secs(10);

    // `None` when the connection did not arrive over a direct IP path, which cannot
    // happen while no relay is configured — but a caller that received it must not
    // advertise anything, and an address it cannot vouch for is worse than none.
    pub async fn observe(
        endpoint: &Endpoint,
        registry: EndpointAddr,
    ) -> Result<Option<SocketAddr>, PeerError> {
        let conn = endpoint
            .connect(registry, Self::ALPN)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        // Nothing to say. Finishing the send side is what tells the far side the
        // request is complete; without it the responder waits on a stream that was
        // opened and never written to.
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        let len = Framing::payload_len(&header)?;
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        // Closing explicitly is what releases the far side. It waits on this
        // connection until the answer has been taken, because finishing a stream
        // marks it complete without waiting for delivery — a responder that simply
        // returned would drop the connection out from under the reply.
        conn.close(0u32.into(), b"observed");

        Ok(Framing::decode(&payload)?)
    }

    // The registry's half.
    //
    // The observed address is taken as a parameter rather than read from the
    // connection: iroh exposes it on `Incoming`, before the handshake completes, and
    // it is gone by the time there is a `Connection` to read it from. The caller
    // captures it and hands it over.
    pub async fn reply_to(
        conn: &Connection,
        observed: IncomingAddr,
    ) -> Result<(), PeerError> {
        let observed = match observed {
            IncomingAddr::Ip(addr) => Some(addr),
            _ => None,
        };

        let (mut send, _recv) = conn
            .accept_bi()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        send.write_all(&Framing::encode(&observed)?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        // Held until the asker closes. `finish` marks the stream complete rather than
        // delivered, so returning here would drop the connection while the reply was
        // still in flight and the asker would see the connection lost instead.
        conn.closed().await;

        Ok(())
    }
}
