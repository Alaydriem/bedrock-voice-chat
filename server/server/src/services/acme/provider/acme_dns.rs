use anyhow::{Result, anyhow};
use serde_json::json;

/// acme-dns (joohoi/acme-dns) integration: the operator pre-registers an
/// account and CNAMEs `_acme-challenge.<domain>` at the delegated subdomain,
/// so this credential can only ever touch one TXT record.
pub struct AcmeDnsProvider {
    server_url: String,
    username: String,
    password: String,
    subdomain: String,
    client: reqwest::Client,
}

impl AcmeDnsProvider {
    pub fn new(server_url: &str, username: &str, password: &str, subdomain: &str) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            subdomain: subdomain.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn publish_txt(&self, _domain: &str, value: &str) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/update", self.server_url))
            .header("X-Api-User", &self.username)
            .header("X-Api-Key", &self.password)
            .json(&json!({
                "subdomain": self.subdomain,
                "txt": value,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("acme-dns update failed ({status}): {body}"));
        }
        Ok(())
    }

    /// acme-dns keeps a two-value rotation internally; there is nothing to
    /// delete.
    pub async fn cleanup_txt(&self, _domain: &str) -> Result<()> {
        Ok(())
    }
}
