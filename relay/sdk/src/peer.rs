use std::sync::Arc;

use bvc_relay::peer::session::{PeerSession, SessionConfig};

use crate::config::SdkConfig;
use crate::error::SdkError;
use crate::frame::SdkFrame;

// The whole SDK, as one object.
//
// `next_frame` is awaited rather than delivered by callback: at roughly fifty
// frames a second per speaker a timer is either a busy loop or latency on every
// frame, and a callback held across the FFI boundary is a pointer this side does
// not own.
//
// `shutdown` exists because uniffi supports no cancellation. A consumer parked
// in `next_frame` is not released by cancelling its coroutine, so without this a
// plugin shutting down hangs on a parked call.
//
// Named `shutdown` rather than `close` because uniffi gives every exported
// object an `AutoCloseable.close()` that frees the handle. A second `close`
// differing only by being suspending is ambiguous at a Kotlin call site inside a
// coroutine, and the two mean different things: one ends the session, the other
// releases the object.
#[derive(uniffi::Object)]
pub struct BvcPeer {
    session: Arc<PeerSession>,
}

#[uniffi::export(async_runtime = "tokio")]
impl BvcPeer {
    // Succeeds whether or not the peer answers. A bridge normally loads before
    // the server it talks to, and failing here would make a startup ordering
    // detail look like a configuration error.
    #[uniffi::constructor]
    pub async fn open(config: SdkConfig) -> Result<Arc<Self>, SdkError> {
        let session = PeerSession::open(SessionConfig {
            node_dir: config.node_dir,
            peerlink: config.peerlink,
            worlds: config.worlds,
            relay_url: config.relay_url,
            inbox_capacity: config.inbox_capacity.max(1) as usize,
        })
        .await
        .map_err(|e| SdkError::Open {
            reason: e.to_string(),
        })?;

        Ok(Arc::new(Self { session }))
    }

    // `None` means the session is closed and will yield nothing further. It does
    // not mean idle: a live session with no speakers parks here.
    pub async fn next_frame(&self) -> Option<SdkFrame> {
        self.session.next().await.map(SdkFrame::from)
    }

    pub fn send(&self, frame: SdkFrame) -> Result<(), SdkError> {
        self.session
            .send(frame.into())
            .map_err(|_| SdkError::NotConnected)
    }

    // This bridge's own link, for the operator to paste into the server's
    // config.hcl.
    pub async fn peerlink(&self) -> Result<String, SdkError> {
        self.session
            .peerlink()
            .await
            .map_err(|e| SdkError::PeerLink {
                reason: e.to_string(),
            })
    }

    pub fn is_connected(&self) -> bool {
        self.session.is_connected()
    }

    pub async fn shutdown(&self) {
        self.session.close().await;
    }
}
