use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a reachability report means for a player about to commit to a server.
///
/// Derived in one place so every surface that shows it — the login address field, the
/// server selector — reaches the same conclusion from the same evidence.
///
/// A failed DNS lookup is deliberately not here: no report exists in that case, so it
/// surfaces as an error from the planner rather than as a verdict about a host that
/// was never contacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ReachabilityVerdict {
    /// A QUIC endpoint answered. Voice can connect.
    Ready,
    /// No QUIC endpoint answered and the WebSocket voice transport did: the network
    /// drops UDP, and voice reaches this server over TCP instead. Connectable, and
    /// separate from `Ready` because the path costs latency the player can hear.
    VoiceFallback,
    /// HTTPS answered, and neither voice transport did. A network that permits TCP
    /// and drops UDP, against a server with no TCP voice path — or one that has none
    /// on this port. The distinction from `Unreachable` matters because it sends the
    /// player to their firewall rather than to the server operator.
    VoiceBlocked,
    /// Nothing answered on either transport. The host is down, or it is the wrong
    /// host.
    Unreachable,
    /// The local stack had no route to any candidate, so nothing about the
    /// destination was learned.
    NoRoute,
}
