use common::Game;

// Classification of an accepted QUIC connection's mTLS-authenticated identity
// (the client certificate's Common Name).
//
// A player cert CN is `{game}:{playername}` (issued by
// `CertificateService::sign_player_cert`). A peered remote server connects with
// `server::{host}:{port}` (issued by `sign_peer_cert`); the `host:port` makes two
// endpoints on one host distinct and is the key the relay `PeerManager` uses for
// dial/accept dedup and tiebreak.
//
// Anything that is neither a well-formed peer marker nor a known-game player CN is
// `Rejected` and the connection is refused. Because both shapes are explicit, a peer
// CN can never be mistaken for a player and a malformed CN can never slip onto the
// player path.
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
    // A normal player. `game` and `name` are split from the CN so the game drives
    // membership keying and the bare name is what inbound packets are stamped with.
    Player {
        game: Game,
        name: String,
    },
    // An identity that is neither a well-formed peer endpoint nor a known-game
    // player CN. The connection is refused (fail closed).
    Rejected {
        identity: String,
    },
}
