use super::{ReceiveError, SendOutcome};
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::s2n_quic::provider::datagram::default::DatagramError;
use std::sync::Arc;

mod recv_datagram;

use recv_datagram::RecvDatagram;


/// The transport one voice session runs over.
///
/// Both halves of a session reach the wire through this: the input loop's reads, its
/// HealthCheck echo and version-gate reject, and the output loop's fan-out writes. The
/// routing, identity and cache logic above it never learns which transport carried a
/// packet, which is what allows one session implementation to serve both.
#[derive(Clone)]
pub(crate) enum SessionLink {
    Quic(Arc<Connection>),
    WebSocket(super::ws::WsLink),
}

impl SessionLink {
    pub(crate) async fn recv(&self) -> Result<Bytes, ReceiveError> {
        match self {
            Self::Quic(connection) => {
                RecvDatagram {
                    conn: connection.as_ref(),
                }
                .await
            }
            Self::WebSocket(link) => link.recv().await,
        }
    }

    pub(crate) fn send(&self, payload: Bytes) -> SendOutcome {
        match self {
            Self::Quic(connection) => Self::send_datagram(connection, payload),
            Self::WebSocket(link) => link.send(payload),
        }
    }

    pub(crate) fn send_batch(&self, payloads: &mut Vec<Bytes>) -> SendOutcome {
        match self {
            Self::Quic(connection) => Self::send_datagram_batch(connection, payloads),
            Self::WebSocket(link) => {
                for payload in payloads.drain(..) {
                    match link.send(payload) {
                        SendOutcome::Ok => {}
                        other => return other,
                    }
                }
                SendOutcome::Ok
            }
        }
    }

    /// The session's device id, which is also its key in `ConnectionRegistry`.
    ///
    /// Read from the transport rather than declared by the client, which is what makes one
    /// player's two devices independently addressable without either impersonating the other.
    pub(crate) fn device(&self) -> u64 {
        match self {
            Self::Quic(connection) => connection.id(),
            Self::WebSocket(link) => link.device(),
        }
    }

    fn send_datagram(connection: &Connection, payload: Bytes) -> SendOutcome {
        let send_res = connection.datagram_mut(
            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                dg.send_datagram(payload)
            },
        );

        match send_res {
            Ok(Ok(())) => SendOutcome::Ok,
            Ok(Err(e)) => Self::classify_datagram_error(e),
            Err(e) => SendOutcome::Fatal(e.to_string()),
        }
    }

    // One lock acquisition and one connection wakeup for the whole batch. Stops at the
    // first error: a full send queue fails the rest of the batch the same way, and a
    // closed connection ends the session regardless. Dropped datagrams read as loss at
    // the receiver, which is the accepted semantics of the bounded audio path.
    fn send_datagram_batch(connection: &Connection, payloads: &mut Vec<Bytes>) -> SendOutcome {
        let send_res = connection.datagram_mut(
            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                let mut result = Ok(());
                for payload in payloads.drain(..) {
                    if let Err(e) = dg.send_datagram(payload) {
                        result = Err(e);
                        break;
                    }
                }
                result
            },
        );

        match send_res {
            Ok(Ok(())) => SendOutcome::Ok,
            Ok(Err(e)) => Self::classify_datagram_error(e),
            Err(e) => SendOutcome::Fatal(e.to_string()),
        }
    }

    // Matched on the variant rather than on `Display`. The two are not interchangeable
    // here: `ConnectionError` renders as "Connection-level error occurred.", which carries
    // neither "closed" nor "reset", so a text match read a dead connection as a transient
    // error and left the send loop running against it.
    //
    // The enum and every variant are `#[non_exhaustive]`, hence the `{ .. }` patterns and
    // the catch-all. A variant added upstream is treated as transient, which costs a few
    // dropped datagrams rather than a session closed on a guess.
    fn classify_datagram_error(error: DatagramError) -> SendOutcome {
        let emsg = error.to_string();
        match error {
            DatagramError::ConnectionError { error, .. } => {
                SendOutcome::ConnectionClosed(error.to_string())
            }
            DatagramError::QueueAtCapacity { .. } => SendOutcome::Capacity(emsg),
            DatagramError::ExceedsPeerTransportLimits { .. } => SendOutcome::Other(emsg),
            _ => SendOutcome::Other(emsg),
        }
    }
}
