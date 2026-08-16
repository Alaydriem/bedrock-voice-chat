use super::ReceiveError;
use bytes::Bytes;
use common::s2n_quic::Connection;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

// Awaits exactly one datagram from a QUIC connection. A hand-written future rather than a
// `futures` dependency, because this is the only place the crate needs one.
pub(super) struct RecvDatagram<'c> {
    pub(super) conn: &'c Connection,
}

impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, ReceiveError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(ReceiveError::Closed {
                detail: e.to_string(),
            })),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(ReceiveError::Unavailable {
                detail: e.to_string(),
            })),
        }
    }
}
