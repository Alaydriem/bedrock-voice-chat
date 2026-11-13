use std::net::SocketAddr;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::config::{PlayCaEndpoint, PlayConfig};

// Resolves the play CA's fixed name to the configured endpoint, again for every new
// connection, so a CA container that restarts with another address is still reached. The
// client that uses it talks to the CA and nothing else, so every other name is refused
// rather than sent to public DNS.
pub struct PlayCaResolver {
    endpoint: PlayCaEndpoint,
}

impl PlayCaResolver {
    pub fn new(endpoint: PlayCaEndpoint) -> Self {
        Self { endpoint }
    }
}

impl Resolve for PlayCaResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let endpoint = self.endpoint.clone();
        let requested = name.as_str().to_string();

        Box::pin(async move {
            if !requested.eq_ignore_ascii_case(PlayConfig::CA_HOST) {
                return Err(format!(
                    "the play CA client resolves only {}, not {requested}",
                    PlayConfig::CA_HOST
                )
                .into());
            }

            let addresses: Vec<SocketAddr> =
                tokio::net::lookup_host((endpoint.host(), endpoint.port()))
                    .await?
                    .collect();
            if addresses.is_empty() {
                return Err(format!("play.ca.connect {endpoint} resolved to no address").into());
            }

            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}
