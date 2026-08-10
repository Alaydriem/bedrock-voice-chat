/// Which transport a spawned client should carry its voice session over.
///
/// Chosen per client because the server's advertised configuration is shared by everyone
/// connected to it, and a mixed-transport channel cannot be expressed that way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Quic,
    WebSocket,
}
