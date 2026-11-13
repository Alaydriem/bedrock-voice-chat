use std::sync::Arc;

use anyhow::Result;
use common::structs::relay::enroll::PlayPreflightOutcome;

use super::ScriptedPlayAuthority;
use crate::services::{CurrentNonce, RelayEnrollmentClient};

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

    /// Asks the registry to prove the internet can reach this server. The registry's nonce is
    /// set on `nonce` as soon as it arrives, so the responder can serve it to the probe.
    pub async fn preflight(
        &self,
        address: Option<&str>,
        port: u16,
        nonce: &CurrentNonce,
    ) -> Result<PlayPreflightOutcome> {
        match self {
            Self::Relay(client) => Ok(client
                .play_preflight(address, port, |value| nonce.set(value))
                .await?),
            Self::Scripted(scripted) => scripted.preflight().await,
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
