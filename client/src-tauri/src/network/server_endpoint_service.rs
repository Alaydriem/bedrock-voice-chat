use std::sync::Arc;
use std::time::Duration;

use common::net::srv::{SrvResolution, SrvResolver};
use common::structs::network::{EndpointSource, ServerAddress, ServerEndpoint};
use moka::future::Cache;
use tauri_plugin_curia::curia;

use super::{CachedEndpoint, DefaultPortProbe, ServerEndpointExpiry};

/// Finds the port a saved server listens on, and remembers it for a short while.
///
/// The saved string stays the identity of the server everywhere. This turns it into the
/// endpoint a request is sent to.
pub struct ServerEndpointService {
    resolver: SrvResolver,
    probe: DefaultPortProbe,
    cache: Cache<String, CachedEndpoint>,
    // Host to the default port that accepted. Outlives `cache`: a probe can cost the whole
    // budget (Windows retries a refused connect for about 2 s), and a server does not move
    // its API between connects.
    probed: Cache<String, u16>,
}

impl ServerEndpointService {
    const CAPACITY: u64 = 64;
    const PROBED_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);

    pub fn new(resolver: SrvResolver, probe: DefaultPortProbe) -> Self {
        Self {
            resolver,
            probe,
            cache: Cache::builder()
                .max_capacity(Self::CAPACITY)
                .expire_after(ServerEndpointExpiry)
                .build(),
            probed: Cache::builder()
                .max_capacity(Self::CAPACITY)
                .time_to_live(Self::PROBED_LIFETIME)
                .build(),
        }
    }

    pub fn new_shared(resolver: SrvResolver, probe: DefaultPortProbe) -> Arc<Self> {
        Arc::new(Self::new(resolver, probe))
    }

    /// The endpoint for a saved server, from the cache while the entry lives.
    pub async fn resolve(&self, identity: &str) -> Result<ServerEndpoint, String> {
        if let Some(cached) = self.cache.get(identity).await {
            return Ok(cached.endpoint);
        }
        self.refresh(identity).await
    }

    /// The base URL for a saved server, or the string as typed when it cannot be parsed.
    ///
    /// For the sign-in check, whose failure report is built from the request that the typed
    /// string produces. An unparsable address fails there as it always did.
    pub async fn resolve_base_or_identity(&self, identity: &str) -> String {
        match self.resolve(identity).await {
            Ok(endpoint) => endpoint.base_url(),
            Err(_) => identity.to_string(),
        }
    }

    /// Resolves again and replaces the cache entry. Called at the start of a session.
    pub async fn refresh(&self, identity: &str) -> Result<ServerEndpoint, String> {
        let address = ServerAddress::parse(identity).map_err(|e| e.to_string())?;
        let resolution = self.resolver.resolve(&address).await;
        let lifetime = Self::lifetime(&resolution);
        let endpoint = self.probe_default(resolution.endpoint).await;
        self.cache
            .insert(
                identity.to_string(),
                CachedEndpoint {
                    endpoint: endpoint.clone(),
                    lifetime,
                },
            )
            .await;
        Ok(endpoint)
    }

    // A `Default` endpoint is only a guess at 443. The port the probe finds replaces it; when
    // no default port accepts, 443 stays, so the failure a player sees is unchanged. Only a
    // port that accepted is remembered, so a server that was down is probed again.
    async fn probe_default(&self, endpoint: ServerEndpoint) -> ServerEndpoint {
        if endpoint.source() != EndpointSource::Default {
            return endpoint;
        }
        let host = endpoint.address().host().to_string();
        let port = match self.probed.get(&host).await {
            Some(port) => Some(port),
            None => {
                let picked = self.probe.pick(&host).await;
                if let Some(port) = picked {
                    self.probed.insert(host.clone(), port).await;
                    curia::info!("no port given; using the default port that accepted", {
                        host: host,
                        port: port,
                    });
                }
                picked
            }
        };
        match port {
            Some(port) if port != endpoint.port() => {
                ServerEndpoint::new(endpoint.address().clone(), port, EndpointSource::Default)
            }
            _ => endpoint,
        }
    }

    /// How long a resolution may be reused: a record's TTL clamped to 30-300 s, 30 s for the
    /// default port, 300 s for a typed port.
    pub fn lifetime(resolution: &SrvResolution) -> Duration {
        match resolution.endpoint.source() {
            EndpointSource::Explicit => ServerEndpointExpiry::MAX_LIFETIME,
            EndpointSource::Default => ServerEndpointExpiry::MIN_LIFETIME,
            EndpointSource::Srv => resolution
                .ttl
                .unwrap_or(ServerEndpointExpiry::MIN_LIFETIME)
                .clamp(
                    ServerEndpointExpiry::MIN_LIFETIME,
                    ServerEndpointExpiry::MAX_LIFETIME,
                ),
        }
    }
}
