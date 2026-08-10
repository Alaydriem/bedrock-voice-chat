/// What became of one outbound payload on a session link.
///
/// Transports fail in the same shapes but report them in different vocabularies, so the
/// classification happens inside the link and callers above it branch on meaning rather
/// than on a transport's error text.
pub(crate) enum SendOutcome {
    Ok,
    /// The peer is gone. The caller stops rather than retrying.
    ConnectionClosed(String),
    /// The send queue is full. The payload is lost, the session is not.
    Capacity(String),
    Other(String),
    /// The link itself could not be queried, which is not a per-payload failure.
    Fatal(String),
}
