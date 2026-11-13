use std::sync::Arc;

use common::curia;
use reqwest::{Certificate, Client};

use crate::config::PlayCaConfig;

use super::ca_resolver::PlayCaResolver;
use super::error::PlayError;
use super::one_time_token::OneTimeToken;

// Signs through the private step-ca.
//
// The intermediate key never leaves step-ca. This holds only the provisioner key, which
// can ask for a certificate for a name but cannot mint a chain.
pub struct StepCaSigner {
    config: PlayCaConfig,
    client: Client,
}

impl StepCaSigner {
    // A transport failure is retried once, with a fresh token.
    const ATTEMPTS: usize = 2;

    pub fn new(config: &PlayCaConfig) -> Result<Self, PlayError> {
        let root = std::fs::read(&config.root).map_err(|e| {
            PlayError::Signer(format!("reading the play root {}: {e}", config.root))
        })?;
        let root = Certificate::from_pem(&root)
            .map_err(|e| PlayError::Signer(format!("the play root is not a certificate: {e}")))?;

        // Only the play root is trusted: the BVC root is never added to a general store,
        // and nothing else may stand in for the CA.
        let client = Client::builder()
            .tls_built_in_root_certs(false)
            .add_root_certificate(root)
            .dns_resolver(Arc::new(PlayCaResolver::new(config.connect.clone())))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| PlayError::Signer(format!("building the CA client: {e}")))?;

        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    // The leaf followed by the intermediate.
    pub async fn sign(&self, csr_pem: &str, fqdn: &str) -> Result<String, PlayError> {
        let mut last = None;

        for _ in 0..Self::ATTEMPTS {
            // Every attempt, a new token: step-ca spends one even on a refusal.
            let token = OneTimeToken::mint(&self.config, fqdn)?;

            let response = match self
                .client
                .post(OneTimeToken::audience(&self.config))
                .json(&serde_json::json!({ "csr": csr_pem, "ott": token }))
                .send()
                .await
            {
                Ok(response) => response,
                Err(e) => {
                    curia::warn!("Play CA unreachable", { "error": e.to_string() });
                    last = Some(PlayError::Signer(format!("reaching the play CA: {e}")));
                    continue;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(PlayError::Signer(format!(
                    "the play CA answered {status}: {body}"
                )));
            }

            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PlayError::Signer(format!("reading the play CA reply: {e}")))?;

            return Self::chain(&body);
        }

        Err(last.unwrap_or_else(|| PlayError::Signer("no signing attempt was made".to_string())))
    }

    fn chain(body: &serde_json::Value) -> Result<String, PlayError> {
        let leaf = body["crt"]
            .as_str()
            .ok_or_else(|| PlayError::Signer("the play CA reply holds no certificate".to_string()))?;
        let intermediate = body["ca"].as_str().unwrap_or_default();

        let mut chain = leaf.trim_end().to_string();
        chain.push('\n');
        if !intermediate.is_empty() {
            chain.push_str(intermediate.trim_end());
            chain.push('\n');
        }

        Ok(chain)
    }
}
