use std::sync::OnceLock;

// Per-connection storage for the mTLS-verified certificate Common Name, written
// once by `PeerIdentityCapture` during the handshake and read back by the accept
// loop through `Connection::query_event_context`.
#[derive(Default)]
pub struct PeerIdentityContext {
    common_name: OnceLock<String>,
    // The verified leaf DER. Held rather than a derived value because the accept loop needs
    // both the fingerprint and the identity, and this is the only point at which the chain
    // is reachable at all.
    leaf_der: OnceLock<Vec<u8>>,
}

impl PeerIdentityContext {
    pub fn set_cn(&self, common_name: String) {
        let _ = self.common_name.set(common_name);
    }

    pub fn cn(&self) -> Option<String> {
        self.common_name.get().cloned()
    }

    pub fn set_leaf_der(&self, leaf_der: Vec<u8>) {
        let _ = self.leaf_der.set(leaf_der);
    }

    pub fn leaf_der(&self) -> Option<Vec<u8>> {
        self.leaf_der.get().cloned()
    }
}
