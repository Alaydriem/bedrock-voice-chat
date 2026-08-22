/// A session was refused because the server already holds as many voice sessions as its
/// operator permits.
///
/// Carries the limit so the client can name the number rather than reporting a bare
/// failure, which reads as a fault rather than as a full server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("server at capacity ({limit} connections)")]
pub struct AtCapacity {
    pub limit: u32,
}
