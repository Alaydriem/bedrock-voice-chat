// Classification of an accepted QUIC connection's authenticated identity.
//
// A normal player connects with an mTLS/application identity of the shape
// `{game}:{playername}` (issued by `CertificateService::sign_player_cert`).
// A peered remote server connects as a client whose identity is
// `{host}:{port}` (the peer's advertised HTTPS endpoint port, issued by
// `sign_peer_cert`) — the port makes two endpoints on one host distinct and is
// the key the relay `PeerManager` uses for dial/accept dedup and tiebreak.
//
// The discriminator is therefore: does the identity end in a numeric port whose
// prefix is NOT a known game keyword? If so it is a `Peer`; otherwise it is a
// `Player` (the safe default — anything malformed or ambiguous takes the normal
// client path, so a relay mis-classification can never silently swallow a real
// player's connection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionKind {
    // A peered remote server. `endpoint` is the canonical `host:port` key
    // (advertised HTTPS endpoint) expected by `PeerManager::register_inbound`.
    Peer { host: String, port: u16, endpoint: String },
    // A normal player (or anything that is not unambiguously a peer).
    Player { identity: String },
}
