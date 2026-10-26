use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Why the sign-in screen's check of a server address did not reach a BVC server.
///
/// Classified from the error itself rather than its message, because the message of an OS
/// error arrives in the player's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ServerCheckFailure {
    /// DNS has no record of the name.
    NameNotFound,
    /// The host answered and nothing was listening on the port.
    Refused,
    /// Nothing answered within the budget.
    TimedOut,
    /// A connection opened and was closed before the exchange finished.
    ConnectionLost,
    /// A server answered with a certificate issued to other names: usually the game host or a
    /// hosting panel, typed in place of the voice server.
    WrongCertificateName { presented: Vec<String> },
    /// The TLS handshake failed for any other reason.
    TlsRejected,
    /// Something answered over HTTPS, and it was not a BVC server's configuration.
    NotBvc { status: Option<u16> },
    /// The address answered with a server error or a rate limit: usually a proxy in front of a
    /// BVC server that is restarting. The address is right; trying later is the fix.
    ServerError { status: u16 },
    /// A configuration came back that this build cannot read: the server is on a newer shape.
    UnreadableConfig,
    Other,
}
