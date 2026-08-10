mod link_error;
mod recv_failure;
mod ws;

#[cfg(debug_assertions)]
mod permissive_server_verifier;

pub(crate) use link_error::VoiceLinkError;
pub(crate) use recv_failure::RecvFailure;
pub(crate) use ws::WsLink;

#[cfg(debug_assertions)]
pub(crate) use permissive_server_verifier::PermissiveServerVerifier;

use bytes::Bytes;
use common::s2n_quic::Connection;
use common::s2n_quic::provider::datagram::default::DatagramError;
use common::structs::network::QuicCloseCode;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

// Awaits exactly one datagram from a QUIC connection. A hand-written future rather than a
// `futures` dependency on the hot path.
struct RecvDatagram<'c> {
    conn: &'c Connection,
}

impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, RecvFailure>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(RecvFailure::Datagram(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(RecvFailure::Query(e.to_string()))),
        }
    }
}

/// The transport this client's voice session runs over.
///
/// Both directions of a session reach the wire through this, so the audio pipeline, the
/// health monitor and the control plane never learn which transport carried a packet.
#[derive(Clone)]
pub(crate) enum DatagramLink {
    Quic(Arc<Connection>),
    WebSocket(WsLink),
}

impl DatagramLink {
    pub(crate) async fn recv(&self) -> Result<Bytes, RecvFailure> {
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

    pub(crate) fn send(&self, payload: Bytes) -> Result<(), VoiceLinkError> {
        match self {
            Self::Quic(connection) => connection
                .datagram_mut(
                    |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                        dg.send_datagram(payload)
                    },
                )
                .map_err(|e| VoiceLinkError::Quic {
                    detail: e.to_string(),
                })?
                .map_err(|e| VoiceLinkError::Quic {
                    detail: e.to_string(),
                }),
            Self::WebSocket(link) => link.send(payload),
        }
    }

    /// Whether the server refused this session's identity, which means stop reconnecting
    /// rather than retry.
    ///
    /// On QUIC the close code surfaces in two places depending on timing: the datagram
    /// error carries it when a receive was in flight as the close landed, but a refusal
    /// issued at `accept()` arrives before the first poll and is only visible by querying
    /// the connection handle. Both are checked, because the server refuses before the
    /// client has sent anything.
    ///
    /// A WebSocket session cannot report one. It is refused during the upgrade, before
    /// there is a session to close, so the failure surfaces to the dialler instead.
    pub(crate) fn is_refused(&self, failure: &RecvFailure) -> bool {
        let Self::Quic(connection) = self else {
            return false;
        };

        if let RecvFailure::Datagram(DatagramError::ConnectionError { error, .. }) = failure
            && Self::is_unauthorized_close(error)
        {
            return true;
        }

        match connection.application_protocol() {
            Err(e) => Self::is_unauthorized_close(&e),
            Ok(_) => false,
        }
    }

    fn is_unauthorized_close(error: &common::s2n_quic::connection::Error) -> bool {
        match error {
            common::s2n_quic::connection::Error::Application { error, .. } => {
                QuicCloseCode::from_u64(u64::from(*error)) == Some(QuicCloseCode::Unauthorized)
            }
            _ => false,
        }
    }
}
