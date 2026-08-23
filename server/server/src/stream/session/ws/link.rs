use crate::stream::session::{ReceiveError, SendOutcome};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// One WebSocket session's two directions.
///
/// The socket itself is split by the listener: a read pump feeds `inbound`, and a write
/// pump drains `outbound`. Neither half is touched here, which is what lets this be
/// `Clone` and shared between the input and output streams the way an `Arc<Connection>`
/// is on the QUIC side.
#[derive(Clone)]
pub(crate) struct WsLink {
    device: u64,
    // Only the input stream reads, so the mutex is never contended. It exists to satisfy
    // the shared-ownership shape the QUIC link already has, not to arbitrate.
    inbound: Arc<Mutex<mpsc::Receiver<Bytes>>>,
    outbound: mpsc::Sender<Bytes>,
}

impl WsLink {
    pub(crate) fn new(
        device: u64,
        inbound: mpsc::Receiver<Bytes>,
        outbound: mpsc::Sender<Bytes>,
    ) -> Self {
        Self {
            device,
            inbound: Arc::new(Mutex::new(inbound)),
            outbound,
        }
    }

    pub(crate) async fn recv(&self) -> Result<Bytes, ReceiveError> {
        self.inbound
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| ReceiveError::Closed {
                detail: "the websocket read pump ended".to_string(),
            })
    }

    pub(crate) fn send(&self, payload: Bytes) -> SendOutcome {
        match self.outbound.try_send(payload) {
            Ok(()) => SendOutcome::Ok,
            // The write pump bounds its own queue and drops the oldest frame when it
            // overflows. Reaching this means even the handoff is saturated, which is the
            // same "shed it and keep the session" case a full QUIC send queue is.
            Err(mpsc::error::TrySendError::Full(_)) => {
                SendOutcome::Capacity("websocket send queue full".to_string())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                SendOutcome::ConnectionClosed("websocket write pump stopped".to_string())
            }
        }
    }

    pub(crate) fn device(&self) -> u64 {
        self.device
    }
}
