use std::sync::Arc;

use anyhow::Result;

use super::ScriptedPlayAuthority;
use crate::services::RelayEnrollmentClient;

/// Where a play name and its certificates come from.
///
/// The registry in production. The scripted variant lets a test stand in for it.
pub enum PlayAuthority {
    Relay(Arc<RelayEnrollmentClient>),
    Scripted(ScriptedPlayAuthority),
}

impl PlayAuthority {
    /// The fully qualified play name this node holds, claiming one if it holds none.
    pub async fn claim(&self) -> Result<String> {
        match self {
            Self::Relay(client) => Ok(client.claim_play_name().await?),
            Self::Scripted(scripted) => scripted.claim().await,
        }
    }

    /// The signed chain, as PEM and leaf first, for a certificate request in DER.
    pub async fn issue(&self, csr_der: Vec<u8>) -> Result<String> {
        match self {
            Self::Relay(client) => Ok(client.request_play_certificate(csr_der).await?),
            Self::Scripted(scripted) => scripted.issue(csr_der).await,
        }
    }
}
