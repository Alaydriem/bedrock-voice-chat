pub mod backoff;
pub mod config;
pub mod inbox;

pub use backoff::Backoff;
pub use config::SessionConfig;
pub use inbox::Inbox;

use std::sync::{Arc, Mutex};

use common::structs::relay::wire::datagram::VoiceFrame;
use iroh::EndpointAddr;
use tokio_util::sync::CancellationToken;

use crate::node::{NodeIdentity, PeerTicket, PeerTicketError};

use super::endpoint::PeerEndpoint;
use super::error::PeerError;
use super::handshake::Handshake;
use super::link::PeerLink;

// A peer link a consumer can hold, rather than a connection it has to manage.
//
// Opening succeeds whether or not the peer answers. A bridge loads before the
// server it talks to as often as after, and failing the open for a peer that is
// merely not up yet turns a startup ordering detail into a configuration error
// the operator cannot act on.
pub struct PeerSession {
    endpoint: PeerEndpoint,
    inbox: Arc<Inbox>,
    link: Mutex<Option<PeerLink>>,
    cancel: CancellationToken,
    worlds: Vec<String>,
}

impl PeerSession {
    // One dial attempt. Generous, because a relay-assisted path legitimately takes
    // several round trips to establish, and short enough that a hung dial cannot
    // outlast the backoff schedule it is supposed to be feeding.
    const DIAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

    // How many consecutive failures pass between reports. The first is always
    // reported; this keeps a relay that is down overnight from writing a line every
    // thirty seconds forever.
    const FAILURES_PER_REPORT: u32 = 10;

    pub async fn open(config: SessionConfig) -> Result<Arc<Self>, PeerError> {
        let addr = PeerTicket::parse(&config.peerlink)
            .map_err(|e| PeerError::Bind(format!("peerlink: {e}")))?;

        let identity = NodeIdentity::load_or_create(&config.node_dir)
            .map_err(|e| PeerError::Bind(e.to_string()))?;
        let endpoint = PeerEndpoint::bind(&identity).await?;

        let session = Arc::new(Self {
            endpoint,
            inbox: Arc::new(Inbox::new(config.inbox_capacity)),
            link: Mutex::new(None),
            cancel: CancellationToken::new(),
            worlds: config.worlds,
        });

        session.spawn_dial_loop(addr);
        Ok(session)
    }

    pub async fn next(&self) -> Option<VoiceFrame> {
        self.inbox.next().await
    }

    // A frame sent while disconnected is dropped rather than queued.
    //
    // Voice held through an outage arrives describing a moment that has passed.
    // The caller is told rather than silently succeeding, so it can stop encoding
    // instead of feeding a link that is not there.
    pub fn send(&self, frame: VoiceFrame) -> Result<(), PeerError> {
        let link = self.link.lock().expect("link lock").clone();

        match link {
            Some(link) => link.send(frame),
            None => Err(PeerError::Transport("not connected".to_string())),
        }
    }

    pub async fn peerlink(&self) -> Result<String, PeerTicketError> {
        self.endpoint.ticket().await
    }

    pub fn is_connected(&self) -> bool {
        self.link.lock().expect("link lock").is_some()
    }

    pub async fn close(&self) {
        self.cancel.cancel();
        self.inbox.close();
        *self.link.lock().expect("link lock") = None;
        self.endpoint.close().await;
    }

    fn spawn_dial_loop(self: &Arc<Self>, addr: EndpointAddr) {
        let session = Arc::clone(self);

        tokio::spawn(async move {
            let mut backoff = Backoff::new();
            let mut failures: u32 = 0;

            loop {
                if session.cancel.is_cancelled() {
                    break;
                }

                match session.connect_once(addr.clone()).await {
                    Ok(link) => {
                        backoff.reset();
                        failures = 0;
                        session.pump(link).await;
                    }
                    Err(e) => {
                        // The first failure and then one in every
                        // `FAILURES_PER_REPORT` afterwards. At debug this said nothing
                        // at any level an operator runs, so a relay that was down read
                        // as a bridge sitting idle rather than one retrying.
                        if failures % Self::FAILURES_PER_REPORT == 0 {
                            tracing::warn!(
                                "peer dial failed ({e}); retrying (attempt {})",
                                failures + 1
                            );
                        } else {
                            tracing::debug!("peer dial failed: {e}");
                        }
                        failures += 1;
                    }
                }

                *session.link.lock().expect("link lock") = None;

                let delay = backoff.next_delay();
                tokio::select! {
                    _ = session.cancel.cancelled() => break,
                    _ = tokio::time::sleep(delay) => {}
                }
            }

            // Whatever ended the loop, nothing further will arrive. Closing here
            // is what releases a reader parked on an inbox that will never fill.
            session.inbox.close();
        });
    }

    async fn connect_once(&self, addr: EndpointAddr) -> Result<PeerLink, PeerError> {
        // Bounded, because the dial loop's backoff only runs between attempts. A dial
        // that never returns — a relay that accepts the connection and then answers
        // nothing is the case — parks the loop inside this call, and the retry that
        // was supposed to happen never does. The bridge then looks idle rather than
        // disconnected, with nothing further logged either way.
        let conn = tokio::time::timeout(
            Self::DIAL_TIMEOUT,
            self.endpoint.endpoint().connect(addr, PeerEndpoint::ALPN),
        )
        .await
        .map_err(|_| PeerError::Transport(format!("dial timed out after {:?}", Self::DIAL_TIMEOUT)))?
        .map_err(|e| PeerError::Transport(e.to_string()))?;

        let accepted = Handshake::dial(&conn, self.worlds.clone()).await?;
        let link = PeerLink::establish(conn, accepted.worlds)?;

        *self.link.lock().expect("link lock") = Some(link.clone());
        Ok(link)
    }

    // Reads until the link ends. Returning is what triggers a redial.
    async fn pump(&self, link: PeerLink) {
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => return,
                frame = link.recv() => match frame {
                    Ok(frame) => self.inbox.push(frame),
                    Err(e) => {
                        tracing::info!("peer link ended: {e}");
                        return;
                    }
                }
            }
        }
    }
}
