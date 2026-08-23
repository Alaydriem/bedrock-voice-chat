use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use super::{ConnectionSequence, RoutedPacket};

pub(crate) struct ConnectionEntry {
    // The canonical identity from this connection's mTLS certificate CN, `game:gamertag`. It is
    // the key every other map in this registry uses, so nothing here has to compose or split one.
    //
    // `Arc<str>` because the audio fan-out snapshots every connection's identity on every frame
    // to release the DashMap shard locks before it awaits. Cloning a `String` there allocated once
    // per connection per frame; this makes the same snapshot a refcount increment.
    pub identity: Arc<str>,
    // Outbound sequence for this connection, so a client can derive its own downlink loss from gaps
    // rather than being told by a report that travels the same lossy path.
    pub sequence: Arc<ConnectionSequence>,
    // Precomputed hash of the identity for interaction measurement, so the audio
    // delivery path never hashes a recipient name per frame.
    pub name_hash: u64,
    // The fingerprint this connection's handshake proved. Revocation addresses a live session
    // by this rather than by identity, so one identity holding two connections on two
    // certificates loses only the revoked one.
    pub fingerprint: String,
    pub tx: mpsc::Sender<RoutedPacket>,
    pub connected_at: Instant,
}
