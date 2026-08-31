use iroh::PublicKey;

use common::structs::relay::wire::control::RefuseReason;

use super::redeem_result::RedeemResult;
use super::scope::PeerScope;

// Who may peer, and for what.
//
// The seam between the transport and whoever owns authorization. `None` is a
// refusal — there is no partial answer, and no default that admits.
//
// `declared` is what the dialer says it hosts. An implementation may return it
// unchanged, narrow it, or refuse; what it must not do is answer with worlds
// the dialer never claimed, because the dialer is the only side that knows.
#[async_trait::async_trait]
pub trait PeerAuthority: Send + Sync {
    fn authorize(&self, node: &PublicKey, declared: &[String]) -> Option<PeerScope>;

    /// Redeems a pairing code for a node that holds no grant.
    ///
    /// Async because an implementation backed by a store has to reach it. `authorize`
    /// stays synchronous because it answers on the packet path and must not await.
    ///
    /// The default refuses, so an implementation written before pairing existed cannot
    /// admit an enrolling peer by omission.
    async fn redeem(&self, _node: &PublicKey, _code: &str, _declared: &[String]) -> RedeemResult {
        RedeemResult::Refused(RefuseReason::UnknownCode)
    }
}
