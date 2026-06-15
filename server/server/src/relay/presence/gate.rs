use common::structs::relay::RelayEndpoint;

// Seam for the in-realm presence proof. Before issuing a peer cert or relaying
// audio for a world, the `PeerManager` asks this gate whether the peer has been
// proven live-present in that world. The real, nonce-echo-backed implementation
// lives in the presence prover; `AlwaysProven` keeps the media plane testable
// without the security plane.
pub trait PresenceGate: Send + Sync {
    fn is_proven(&self, peer: &RelayEndpoint, hashed_world: &str) -> bool;
}

// Permissive stub. Treats every peer as proven so the dial/accept and
// forwarding logic can be exercised in isolation.
pub struct AlwaysProven;

impl PresenceGate for AlwaysProven {
    fn is_proven(&self, _peer: &RelayEndpoint, _hashed_world: &str) -> bool {
        true
    }
}

// Deny-all gate, useful for tests asserting that the manager refuses to peer
// when presence is unproven.
pub struct NeverProven;

impl PresenceGate for NeverProven {
    fn is_proven(&self, _peer: &RelayEndpoint, _hashed_world: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep() -> RelayEndpoint {
        RelayEndpoint {
            host: "peer".into(),
            port: 5000,
            primary: false,
        }
    }

    #[test]
    fn always_proven_returns_true() {
        assert!(AlwaysProven.is_proven(&ep(), "hW"));
    }

    #[test]
    fn never_proven_returns_false() {
        assert!(!NeverProven.is_proven(&ep(), "hW"));
    }
}
