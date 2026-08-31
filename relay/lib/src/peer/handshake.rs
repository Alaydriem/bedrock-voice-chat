use common::structs::relay::wire::control::{
    Accept, ControlFrame, Enrol, Enrolled, Hello, Refuse, RefuseReason,
};
use common::structs::relay::wire::{Framing, WireVersion};
use iroh::endpoint::{Connection, RecvStream, SendStream};

use super::authority::PeerAuthority;
use super::error::PeerError;
use super::redeem_result::RedeemResult;

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

        // Two openings reach here. `Hello` is a peer that expects to already hold a
        // grant; `Enrol` is one redeeming a pairing code because it does not.
        let (versions, worlds, code) = match Self::read_frame(&mut recv).await? {
            ControlFrame::Hello(hello) => (hello.versions, hello.worlds, None),
            ControlFrame::Enrol(enrol) => (enrol.versions, enrol.worlds, Some(enrol.code)),
            _ => {
                return Err(PeerError::Unexpected {
                    expected: "Hello or Enrol",
                });
            }
        };

        // Version first: a peer that cannot be spoken to at all is refused before
        // its authorization is considered, so a version problem never reads as an
        // authorization one.
        let Some(version) = WireVersion::negotiate(WireVersion::SUPPORTED, &versions) else {
            Self::refuse(&mut send, RefuseReason::NoCommonVersion).await?;
            return Err(PeerError::Refused(RefuseReason::NoCommonVersion));
        };

        let enrolling = code.is_some();

        let scope = match code {
            None => match authority.authorize(&node, &worlds) {
                Some(scope) => scope,
                None => {
                    Self::refuse(&mut send, RefuseReason::NotAuthorized).await?;
                    return Err(PeerError::Refused(RefuseReason::NotAuthorized));
                }
            },
            Some(code) => match authority.redeem(&node, &code, &worlds).await {
                RedeemResult::Granted(scope) => scope,
                RedeemResult::Refused(reason) => {
                    Self::refuse(&mut send, reason).await?;
                    return Err(PeerError::Refused(reason));
                }
            },
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

        // The answer has to match the question: a dialer that sent `Enrol` is reading
        // `Enrolled`, and one that sent `Hello` is reading `Accept`.
        let frame = if enrolling {
            ControlFrame::Enrolled(Enrolled {
                version: accept.version,
                worlds: accept.worlds.clone(),
                capabilities: accept.capabilities.clone(),
            })
        } else {
            ControlFrame::Accept(accept.clone())
        };

        send.write_all(&Framing::encode(&frame)?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        Ok(accept)
    }

    /// Opens a link by redeeming a pairing code, for a bridge that holds no grant yet.
    ///
    /// Separate from `dial` because the answer differs: this reads `Enrolled`, which tells
    /// the caller a grant was written rather than merely found.
    pub async fn enrol(
        conn: &Connection,
        worlds: Vec<String>,
        code: String,
    ) -> Result<Enrolled, PeerError> {
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let frame = ControlFrame::Enrol(Enrol {
            versions: WireVersion::SUPPORTED.to_vec(),
            worlds,
            code,
        });
        send.write_all(&Framing::encode(&frame)?)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        // The exchange is one frame each way, so the send side is done. Finishing it is
        // what flushes the frame; without this the acceptor waits on a stream that has
        // been opened and never written to.
        send.finish()
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        match Self::read_frame(&mut recv).await? {
            ControlFrame::Enrolled(enrolled) => Ok(enrolled),
            ControlFrame::Refuse(refuse) => Err(PeerError::Refused(refuse.reason)),
            _ => Err(PeerError::Unexpected {
                expected: "Enrolled or Refuse",
            }),
        }
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
