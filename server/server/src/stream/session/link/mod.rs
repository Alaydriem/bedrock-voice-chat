use super::{ReceiveError, SendOutcome};
use bytes::Bytes;
use common::s2n_quic::Connection;
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
            Ok(Err(e)) => {
                let emsg = e.to_string();
                let lower = emsg.to_ascii_lowercase();
                if (lower.contains("connection") && lower.contains("clos"))
                    || lower.contains("closed")
                    || lower.contains("reset")
                {
                    SendOutcome::ConnectionClosed(emsg)
                } else if lower.contains("capacity") || lower.contains("queue") {
                    SendOutcome::Capacity(emsg)
                } else {
                    SendOutcome::Other(emsg)
                }
            }
            Err(e) => SendOutcome::Fatal(e.to_string()),
        }
    }
}
