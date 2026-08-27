use std::sync::Arc;

use anyhow::{Result, anyhow};

use crate::services::RelayEnrollmentClient;

// Publishes the DNS-01 challenge through the relay registry.
//
// There is no credential. The enrollment session authenticates this node
// cryptographically, so the relay resolves the node to its assigned name itself and
// refuses any other — a tighter scope than a shared API key could express.
pub struct BvcRelayProvider {
    client: Arc<RelayEnrollmentClient>,
    name: String,
}

impl BvcRelayProvider {
    pub fn new(client: Arc<RelayEnrollmentClient>, name: String) -> Self {
        Self { client, name }
    }

    // The name is checked here as well as at the relay. The relay's refusal is the
    // security boundary; this one turns a local misconfiguration into an error naming
    // both names rather than a remote refusal naming neither.
    pub async fn publish_txt(&self, domain: &str, value: &str) -> Result<()> {
        if domain != self.name {
            return Err(anyhow!(
                "refusing to publish a challenge for {domain}; this server is {}",
                self.name
            ));
        }

        self.client
            .publish_txt(&self.name, value)
            .await
            .map_err(|e| anyhow!("publishing a challenge through the relay: {e}"))
    }

    // The relay removes every challenge record it holds for a name when the name is
    // withdrawn, and keeps concurrent values apart by record id in the meantime, so
    // there is nothing to ask for here.
    pub async fn cleanup_txt(&self, _domain: &str) -> Result<()> {
        Ok(())
    }
}
