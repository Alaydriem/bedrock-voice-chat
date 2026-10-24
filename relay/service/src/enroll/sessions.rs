use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::structs::relay::enroll::EnrollFrame;
use common::structs::relay::wire::Framing;
use iroh::PublicKey;

use super::session::EnrollSession;

// Every live enrollment session, keyed by node.
//
// Session presence is itself the liveness signal: a server that cannot hold this
// connection open is one the daily challenge would fail anyway.
pub struct EnrollSessions {
    sessions: Mutex<HashMap<PublicKey, EnrollSession>>,
}

impl EnrollSessions {
    // How long a node has to answer a challenge. A session that is held open but never
    // answered would otherwise stall the caller indefinitely, and the daily pass visits
    // registrations one after another: one silent node would stop validation for
    // everyone behind it.
    pub const CHALLENGE_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn insert(&self, session: EnrollSession) {
        self.sessions
            .lock()
            .expect("session table lock")
            .insert(session.node(), session);
    }

    pub fn remove(&self, node_id: &PublicKey) {
        self.sessions
            .lock()
            .expect("session table lock")
            .remove(node_id);
    }

    pub fn contains(&self, node_id: &PublicKey) -> bool {
        self.sessions
            .lock()
            .expect("session table lock")
            .contains_key(node_id)
    }

    pub fn len(&self) -> usize {
        self.sessions.lock().expect("session table lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn nodes(&self) -> Vec<PublicKey> {
        self.sessions
            .lock()
            .expect("session table lock")
            .keys()
            .copied()
            .collect()
    }

    fn get(&self, node_id: &PublicKey) -> Option<EnrollSession> {
        self.sessions
            .lock()
            .expect("session table lock")
            .get(node_id)
            .cloned()
    }

    // Sends a nonce down the held session and returns the signature the node
    // answered with.
    //
    // `None` means the session is gone or the node did not answer, both of which the
    // caller treats as a failed validation. It is deliberately not an error type: a
    // node that cannot be reached and one that answers wrongly are the same outcome
    // to a validation pass, and distinguishing them here would invite a caller to
    // treat one as excusable.
    pub async fn challenge(&self, node_id: &PublicKey, nonce: &[u8]) -> Option<Vec<u8>> {
        let session = self.get(node_id)?;
        tokio::time::timeout(Self::CHALLENGE_TIMEOUT, Self::exchange(&session, nonce))
            .await
            .ok()?
    }

    async fn exchange(session: &EnrollSession, nonce: &[u8]) -> Option<Vec<u8>> {
        let (mut send, mut recv) = session.connection().open_bi().await.ok()?;

        let frame = EnrollFrame::Challenge {
            nonce: nonce.to_vec(),
        };
        send.write_all(&Framing::encode(&frame).ok()?).await.ok()?;
        send.finish().ok()?;

        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header).await.ok()?;
        let len = Framing::payload_len(&header).ok()?;
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload).await.ok()?;

        match Framing::decode::<EnrollFrame>(&payload).ok()? {
            EnrollFrame::ChallengeReply { signature } => Some(signature),
            _ => None,
        }
    }
}

impl Default for EnrollSessions {
    fn default() -> Self {
        Self::new()
    }
}
