use common::structs::relay::Capability;

// What a peer is permitted, as the acceptor decides it.
//
// `worlds` is the negotiated set: the dialer's declaration after any narrowing
// the acceptor applied. Carried rather than referenced so this crate never
// learns where the decision came from: a server reads it from `config.hcl`, and
// a bridge that one day accepts would answer from something else entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerScope {
    pub worlds: Vec<String>,
    pub capabilities: Vec<Capability>,
}
