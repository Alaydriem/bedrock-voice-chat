use serde_json::{Value, json};

use super::error::AcmeError;

const DEFAULT_API_BASE: &str = "https://api.cloudflare.com/client/v4";

// The DNS-01 half of the registry's own issuance.
//
// The zone is discovered from the hostname rather than configured. The API token has
// access to both this project's zones, so asking Cloudflare which zone a name belongs
// to removes a config field that fails obscurely when wrong.
pub struct CloudflareDns {
    http: reqwest::Client,
    api_token: String,
    api_base: String,
}

impl CloudflareDns {
    const TTL_SECONDS: u32 = 60;

    pub fn new(api_token: &str) -> Self {
        Self::new_with_base(api_token, DEFAULT_API_BASE)
    }

    pub fn new_with_base(api_token: &str, api_base: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_token: api_token.to_string(),
            api_base: api_base.trim_end_matches('/').to_string(),
        }
    }

    // Most specific first, stopping at the apex. Walking past it would query the
    // public suffix, which no account owns.
    pub fn zone_candidates(domain: &str) -> Vec<String> {
        let labels: Vec<&str> = domain.split('.').collect();
        (0..labels.len().saturating_sub(1))
            .map(|start| labels[start..].join("."))
            .collect()
    }

    pub fn challenge_name(domain: &str) -> String {
        format!("_acme-challenge.{domain}")
    }

    pub async fn zone_for(&self, domain: &str) -> Result<String, AcmeError> {
        for candidate in Self::zone_candidates(domain) {
            let response: Value = self
                .http
                .get(format!("{}/zones?name={}", self.api_base, candidate))
                .bearer_auth(&self.api_token)
                .send()
                .await
                .map_err(|e| AcmeError::Http(e.to_string()))?
                .json()
                .await
                .map_err(|e| AcmeError::Http(e.to_string()))?;

            if let Some(id) = response["result"]
                .as_array()
                .and_then(|zones| zones.first())
                .and_then(|zone| zone["id"].as_str())
            {
                return Ok(id.to_string());
            }
        }

        Err(AcmeError::NoZone(domain.to_string()))
    }

    pub async fn publish_txt(&self, domain: &str, value: &str) -> Result<(), AcmeError> {
        let zone = self.zone_for(domain).await?;
        let response: Value = self
            .http
            .post(format!("{}/zones/{}/dns_records", self.api_base, zone))
            .bearer_auth(&self.api_token)
            .json(&json!({
                "type": "TXT",
                "name": Self::challenge_name(domain),
                "content": value,
                "ttl": Self::TTL_SECONDS,
            }))
            .send()
            .await
            .map_err(|e| AcmeError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| AcmeError::Http(e.to_string()))?;

        if response["success"] != Value::Bool(true) {
            return Err(AcmeError::Http(format!("record create failed: {response}")));
        }

        Ok(())
    }

    // Every challenge record for the name. A retry leaves two, and removing only one
    // would leave the zone carrying a stale authorization.
    pub async fn cleanup_txt(&self, domain: &str) -> Result<(), AcmeError> {
        let zone = self.zone_for(domain).await?;
        let name = Self::challenge_name(domain);

        let listing: Value = self
            .http
            .get(format!(
                "{}/zones/{}/dns_records?type=TXT&name={}",
                self.api_base, zone, name
            ))
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|e| AcmeError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| AcmeError::Http(e.to_string()))?;

        if let Some(records) = listing["result"].as_array() {
            for record in records {
                if let Some(id) = record["id"].as_str() {
                    let _ = self
                        .http
                        .delete(format!("{}/zones/{}/dns_records/{}", self.api_base, zone, id))
                        .bearer_auth(&self.api_token)
                        .send()
                        .await;
                }
            }
        }

        Ok(())
    }
}
