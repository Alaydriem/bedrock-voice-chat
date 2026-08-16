use iroh::PublicKey;

use super::scope::PeerScope;

// Who may peer, and for what.
//
// The seam between the transport and whoever owns authorization. `None` is a
// refusal — there is no partial answer, and no default that admits.
//
// `declared` is what the dialer says it hosts. An implementation may return it
// unchanged, narrow it, or refuse; what it must not do is answer with worlds
// the dialer never claimed, because the dialer is the only side that knows.
pub trait PeerAuthority: Send + Sync {
    fn authorize(&self, node: &PublicKey, declared: &[String]) -> Option<PeerScope>;
}
