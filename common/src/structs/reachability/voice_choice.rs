/// Which transport a connect should dial, decided from the measurements rather than by
/// dialling both and seeing which lands first.
///
/// A race spends the server a TLS handshake and a registered session for every player who
/// ends up on QUIC anyway, and it holds the connect open while a transport nobody will use
/// finishes. The report already knows which path is better; this is that answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceChoice {
    Quic,
    WebSocket,
    /// Neither transport answered. The connect has nothing to dial and says so, rather
    /// than walking every candidate to rediscover it.
    None,
}
