use crate::stream::quic::connection_registry::ConnectionRegistry;

// Whether this server currently serves a given canonical identity.
//
// A trait rather than a direct `ConnectionRegistry` reference so the ingest
// decision can be tested without a live registry, and so the per-packet cost is
// a single index lookup rather than the set `on_voice_identities` builds.
pub trait LocalClients: Send + Sync {
    fn has_live_client(&self, identity: &str) -> bool;
}

impl LocalClients for ConnectionRegistry {
    fn has_live_client(&self, identity: &str) -> bool {
        ConnectionRegistry::has_live_client(self, identity)
    }
}
