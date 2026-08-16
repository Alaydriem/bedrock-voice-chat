use std::sync::OnceLock;

// Per-connection storage for the mTLS-verified certificate Common Name, written
// once by `PeerIdentityCapture` during the handshake and read back by the accept
// loop through `Connection::query_event_context`.
#[derive(Default)]
pub struct PeerIdentityContext {
    common_name: OnceLock<String>,
}

impl PeerIdentityContext {
    pub fn set_cn(&self, common_name: String) {
        let _ = self.common_name.set(common_name);
    }

    pub fn cn(&self) -> Option<String> {
        self.common_name.get().cloned()
    }
}
