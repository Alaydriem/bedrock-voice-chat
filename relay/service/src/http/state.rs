use std::sync::Arc;

use crate::config::{DiscordConfig, HttpConfig};
use crate::discord::IdentitySource;
use crate::registry::{ClaimService, RegistryService};

// Everything the routes share.
//
// Held rather than rebuilt per request because two of these carry connection pools and
// one carries a Discord credential; constructing them per request would open a pool
// per visitor.
pub struct HttpState {
    pub http: HttpConfig,
    pub discord: DiscordConfig,
    pub registry: Arc<RegistryService>,
    pub claims: Arc<ClaimService>,
    pub identity: IdentitySource,
}

impl HttpState {
    pub fn new(
        http: HttpConfig,
        discord: DiscordConfig,
        registry: Arc<RegistryService>,
        claims: Arc<ClaimService>,
        identity: IdentitySource,
    ) -> Self {
        Self {
            http,
            discord,
            registry,
            claims,
            identity,
        }
    }

    pub fn new_shared(
        http: HttpConfig,
        discord: DiscordConfig,
        registry: Arc<RegistryService>,
        claims: Arc<ClaimService>,
        identity: IdentitySource,
    ) -> Arc<Self> {
        Arc::new(Self::new(http, discord, registry, claims, identity))
    }
}
