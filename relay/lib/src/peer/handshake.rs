use common::structs::relay::wire::control::{Accept, ControlFrame, Hello, Refuse, RefuseReason};
use common::structs::relay::wire::{Framing, WireVersion};
use iroh::endpoint::{Connection, RecvStream, SendStream};

use super::authority::PeerAuthority;
use super::error::PeerError;

// The control-stream exchange that opens a peer link.
//
// The dialer offers every version it speaks and declares the worlds it hosts;
// the acceptor chooses a version, narrows the declaration through whatever
// filter it holds, and echoes the result back. The result is echoed rather than
// assumed because a dialer whose declaration was narrowed otherwise looks
// connected and healthy while every frame it sends for a removed world is
// dropped at the far end.
//
// Both halves live here. A server uses `accept` and `dial`; a bridge uses `dial`.
// Keeping them together is what stops the two ends drifting apart, which is the
// failure a published wire makes expensive.
pub struct Handshake;

impl Handshake {
    pub async fn dial(conn: &Connection, worlds: Vec<String>) -> Result<Accept, PeerError> {
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let hello = ControlFrame::Hello(Hello {
            versions: WireVersion::SUPPORTED.to_vec(),
            worlds,
        });
        send.write_all(&Framing::encode(&hello)?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        // The exchange is one frame each way, so the send side is done. Finishing
        // it is what flushes the frame; without this the acceptor waits on a
        // stream that has been opened and never written to.
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        match Self::read_frame(&mut recv).await? {
            ControlFrame::Accept(accept) => Ok(accept),
            ControlFrame::Refuse(refuse) => Err(PeerError::Refused(refuse.reason)),
            _ => Err(PeerError::Unexpected {
                expected: "Accept or Refuse",
            }),
        }
    }

    pub async fn accept(
        conn: &Connection,
        authority: &dyn PeerAuthority,
    ) -> Result<Accept, PeerError> {
        // Straight from the peer's TLS certificate, so this is the authentication
        // rather than a claim the peer made about itself.
        let node = conn.remote_id();

        let (mut send, mut recv) = conn
            .accept_bi()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let ControlFrame::Hello(hello) = Self::read_frame(&mut recv).await? else {
            return Err(PeerError::Unexpected { expected: "Hello" });
        };

        // Version first: a peer that cannot be spoken to at all is refused before
        // its authorization is considered, so a version problem never reads as an
        // authorization one.
        let Some(version) = WireVersion::negotiate(WireVersion::SUPPORTED, &hello.versions) else {
            Self::refuse(&mut send, RefuseReason::NoCommonVersion).await?;
            return Err(PeerError::Refused(RefuseReason::NoCommonVersion));
        };

        let Some(scope) = authority.authorize(&node, &hello.worlds) else {
            Self::refuse(&mut send, RefuseReason::NotAuthorized).await?;
            return Err(PeerError::Refused(RefuseReason::NotAuthorized));
        };

        // An empty set means the link would carry nothing. Refusing is what
        // keeps that from presenting as a healthy connection.
        if scope.worlds.is_empty() {
            Self::refuse(&mut send, RefuseReason::NoSharedWorld).await?;
            return Err(PeerError::Refused(RefuseReason::NoSharedWorld));
        }

        let accept = Accept {
            version,
            worlds: scope.worlds,
            capabilities: scope.capabilities,
        };
        send.write_all(&Framing::encode(&ControlFrame::Accept(accept.clone()))?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        Ok(accept)
    }

    // Sent before the error is returned, so the dialer learns why rather than
    // seeing a bare close it would read as a network fault and retry.
    async fn refuse(send: &mut SendStream, reason: RefuseReason) -> Result<(), PeerError> {
        let frame = ControlFrame::Refuse(Refuse { reason });
        send.write_all(&Framing::encode(&frame)?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        Ok(())
    }

    // The length is read before any of the payload, and validated against the cap
    // before a byte is allocated for it.
    async fn read_frame(recv: &mut RecvStream) -> Result<ControlFrame, PeerError> {
        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let len = Framing::payload_len(&header)?;
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        Ok(ControlFrame::decode(&payload)?)
    }
}
