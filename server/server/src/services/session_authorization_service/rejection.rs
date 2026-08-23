/// Why a presented certificate may not open a voice session.
///
/// Every variant is a refusal. There is no "unknown" case: a certificate the server cannot
/// reason about is refused rather than admitted, which is what keeps the QUIC and WebSocket
/// handshakes fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRejection {
    /// The leaf could not be parsed, or carried no Common Name.
    Unreadable,
    /// The Common Name is not a known-game player identity.
    NotAPlayer,
    /// No player row matches the certificate's identity.
    UnknownPlayer,
    /// The player exists and is banished.
    Banished,
    /// The certificate itself has been revoked.
    Revoked,
    /// The lookup could not be completed. Refused rather than admitted.
    Unavailable,
}

impl std::fmt::Display for SessionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::Unreadable => "certificate could not be read",
            Self::NotAPlayer => "certificate identity is not a valid player CN",
            Self::UnknownPlayer => "no player matches the certificate identity",
            Self::Banished => "player is banished",
            Self::Revoked => "certificate is revoked",
            Self::Unavailable => "authorization lookup failed",
        };
        f.write_str(reason)
    }
}
