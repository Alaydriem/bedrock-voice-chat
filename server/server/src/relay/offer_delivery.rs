use std::sync::Arc;

use common::structs::relay::RelayEndpoint;

use super::discovery::client::RelayClient;
use super::orchestrator::OfferDelivery;

// Production `OfferDelivery`: for each peer the orchestrator decides to offer to,
// fire a `/relay/offer` at that peer (the minter) asking it to mint a code bound
// to OUR endpoint and inject it into the realm. Non-blocking: each offer is a
// spawned HTTP call so the orchestrator tick never awaits I/O.
pub struct ProductionOfferDelivery {
    relay_client: Arc<RelayClient>,
    self_endpoint: RelayEndpoint,
    // OUR sealed-box public key, advertised in each offer so the minter seals the
    // code to us.
    public_key: Vec<u8>,
}

impl ProductionOfferDelivery {
    pub fn new(
        relay_client: Arc<RelayClient>,
        self_endpoint: RelayEndpoint,
        public_key: Vec<u8>,
    ) -> Self {
        Self {
            relay_client,
            self_endpoint,
            public_key,
        }
    }

    pub fn new_shared(
        relay_client: Arc<RelayClient>,
        self_endpoint: RelayEndpoint,
        public_key: Vec<u8>,
    ) -> Arc<Self> {
        Arc::new(Self::new(relay_client, self_endpoint, public_key))
    }
}

impl OfferDelivery for ProductionOfferDelivery {
    fn send_offer(&self, peer: RelayEndpoint, hashed_world: String) {
        let relay_client = self.relay_client.clone();
        let asker = self.self_endpoint.clone();
        let public_key = self.public_key.clone();
        tokio::spawn(async move {
            if let Err(e) = relay_client
                .offer(&peer.host, peer.port, &hashed_world, &asker, public_key)
                .await
            {
                tracing::warn!(
                    "relay offer to {}:{} for world {} failed: {}",
                    peer.host,
                    peer.port,
                    hashed_world,
                    e
                );
            }
        });
    }
}
