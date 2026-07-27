use anyhow::{Result, anyhow};
use serde_json::json;

const DEFAULT_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// Cloudflare DNS API integration. The token must be scoped to the single
/// zone with DNS-edit permission only — never a global key.
pub struct CloudflareProvider {
    api_token: String,
    api_base: String,
    client: reqwest::Client,
}

impl CloudflareProvider {
    pub fn new(api_token: &str) -> Self {
        Self::new_with_base(api_token, DEFAULT_API_BASE)
    }

    pub fn new_with_base(api_token: &str, api_base: &str) -> Self {
        Self {
            api_token: api_token.to_string(),
            api_base: api_base.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn publish_txt(&self, domain: &str, value: &str) -> Result<()> {
        let zone_id = self.find_zone_id(domain).await?;
        let record_name = format!("_acme-challenge.{domain}");
        let response: serde_json::Value = self
            .client
            .post(format!("{}/zones/{}/dns_records", self.api_base, zone_id))
            .bearer_auth(&self.api_token)
            .json(&json!({
                "type": "TXT",
                "name": record_name,
                "content": value,
                "ttl": 60,
            }))
            .send()
            .await?
            .json()
            .await?;
        if response["success"] != serde_json::Value::Bool(true) {
            return Err(anyhow!("cloudflare record create failed: {response}"));
        }
        Ok(())
    }

    pub async fn cleanup_txt(&self, domain: &str) -> Result<()> {
        let zone_id = self.find_zone_id(domain).await?;
        let record_name = format!("_acme-challenge.{domain}");
        let listing: serde_json::Value = self
            .client
            .get(format!(
                "{}/zones/{}/dns_records?type=TXT&name={}",
                self.api_base, zone_id, record_name
            ))
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .json()
            .await?;
        if let Some(records) = listing["result"].as_array() {
            for record in records {
                if let Some(id) = record["id"].as_str() {
                    let _ = self
                        .client
                        .delete(format!(
                            "{}/zones/{}/dns_records/{}",
                            self.api_base, zone_id, id
                        ))
                        .bearer_auth(&self.api_token)
                        .send()
                        .await;
                }
            }
        }
        Ok(())
    }

    /// Walks the domain's parent labels until a zone matches: for
    /// voice.eu.example.com it tries voice.eu.example.com, eu.example.com,
    /// then example.com.
    async fn find_zone_id(&self, domain: &str) -> Result<String> {
        let labels: Vec<&str> = domain.split('.').collect();
        for start in 0..labels.len().saturating_sub(1) {
            let candidate = labels[start..].join(".");
            let response: serde_json::Value = self
                .client
                .get(format!("{}/zones?name={}", self.api_base, candidate))
                .bearer_auth(&self.api_token)
                .send()
                .await?
                .json()
                .await?;
            if let Some(zone) = response["result"].as_array().and_then(|z| z.first()) {
                if let Some(id) = zone["id"].as_str() {
                    return Ok(id.to_string());
                }
            }
        }
        Err(anyhow!("no cloudflare zone found for domain {domain:?}"))
    }
}
