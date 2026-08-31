use common::structs::relay::wire::control::RefuseReason;

use super::scope::PeerScope;

// What redeeming a pairing code decided.
//
// A refusal carries its reason so a dialer can tell a wrong code from an expired one
// without an operator reading both sides' logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemResult {
    Granted(PeerScope),
    Refused(RefuseReason),
}
