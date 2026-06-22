// Classification of an accepted QUIC connection's authenticated identity.
//
// A normal player connects with an mTLS/application identity of the shape
// `{game}:{playername}` (issued by `CertificateService::sign_player_cert`).
// A peered remote server connects as a client whose identity is
// `server::{host}:{port}` (an explicit peer marker issued by `sign_peer_cert`);
// the `host:port` makes two endpoints on one host distinct and is the key the
// relay `PeerManager` uses for dial/accept dedup and tiebreak.
//
// The `server::` prefix is the discriminator: a CN that carries it is a `Peer`
// when the remainder is a well-formed `host:port`, and is `Rejected` (fail
// closed — connection refused) when it is not. A peer CN therefore can NEVER be
// mistaken for a player and a malformed peer CN can NEVER slip onto the player
// path. Any identity WITHOUT the prefix is a `Player` (the safe default for the
// normal client path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionKind {
    // A peered remote server. `endpoint` is the canonical `host:port` key
    // (advertised HTTPS endpoint) expected by `PeerManager::register_inbound` —
    // the `server::` marker is stripped.
    Peer {
        host: String,
        port: u16,
        endpoint: String,
    },
    // A normal player (any identity without the `server::` peer marker).
    Player {
        identity: String,
    },
    // A `server::`-prefixed identity that is not a well-formed `host:port`. The
    // connection is refused — it is never treated as a player (fail closed).
    Rejected {
        identity: String,
    },
}
