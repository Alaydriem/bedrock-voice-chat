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
    /// HTTPS answered and QUIC did not: a network that permits TCP and drops UDP.
    /// The distinction matters because it sends the player to their firewall rather
    /// than to the server operator.
    VoiceBlocked,
    /// Nothing answered on either transport. The host is down, or it is the wrong
    /// host.
    Unreachable,
    /// The local stack had no route to any candidate, so nothing about the
    /// destination was learned.
    NoRoute,
}
