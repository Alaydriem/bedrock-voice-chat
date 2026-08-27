use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::config::CloudflareConfig;

use super::error::DnsError;

pub struct CloudflareClient {
    http: reqwest::Client,
    api_token: String,
    zone_id: String,
}

impl CloudflareClient {
    // Short enough that a challenge withdrawn after issuance stops resolving while
    // the order is still fresh in an operator's mind, and long enough that a resolver
    // is not asked on every validation attempt.
    const TTL_SECONDS: u32 = 60;

    pub fn new(config: &CloudflareConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_token: config.api_token.clone(),
            zone_id: config.zone_id.clone(),
        }
    }

    pub async fn create(&self, kind: &str, fqdn: &str, content: &str) -> Result<String, DnsError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            self.zone_id
        );

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&json!({
                "type": kind,
                "name": fqdn,
                "content": content,
                "ttl": Self::TTL_SECONDS,
            }))
            .send()
            .await
            .map_err(|e| DnsError::Http(e.to_string()))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| DnsError::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(DnsError::Status {
                status: status.as_u16(),
                body: body.to_string(),
            });
        }

        body.get("result")
            .and_then(|r| r.get("id"))
            .and_then(|id| id.as_str())
            .map(String::from)
            .ok_or(DnsError::MissingRecordId)
    }

    pub async fn delete(&self, record_id: &str) -> Result<(), DnsError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            self.zone_id, record_id
        );

        let response = self
            .http
            .delete(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map_err(|e| DnsError::Http(e.to_string()))?;

        // A record already gone is the desired state. Treating it as an error would
        // strand the ledger row that names it.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(DnsError::Status {
                status: response.status().as_u16(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }
}

// Records what would have been written. The zone is the one thing tests must not
// touch, and asserting on the ledger alone would not catch a writer that never
// called out at all.
pub struct RecordingApi {
    created: Mutex<Vec<(String, String, String)>>,
    deleted: Mutex<Vec<String>>,
}

impl RecordingApi {
    pub fn new() -> Self {
        Self {
            created: Mutex::new(Vec::new()),
            deleted: Mutex::new(Vec::new()),
        }
    }

    pub fn create(&self, kind: &str, fqdn: &str, content: &str) -> String {
        let mut created = self.created.lock().expect("recording lock");
        let id = format!("record-{}", created.len());
        created.push((kind.to_string(), fqdn.to_string(), content.to_string()));
        id
    }

    pub fn delete(&self, record_id: &str) {
        self.deleted
            .lock()
            .expect("recording lock")
            .push(record_id.to_string());
    }

    pub fn created_names(&self) -> Vec<String> {
        self.created
            .lock()
            .expect("recording lock")
            .iter()
            .map(|(_, name, _)| name.clone())
            .collect()
    }

    pub fn live_ids(&self) -> Vec<String> {
        let created = self.created.lock().expect("recording lock");
        let deleted = self.deleted.lock().expect("recording lock");
        (0..created.len())
            .map(|i| format!("record-{i}"))
            .filter(|id| !deleted.contains(id))
            .collect()
    }
}

impl Default for RecordingApi {
    fn default() -> Self {
        Self::new()
    }
}

// Enum delegation rather than a trait object, matching how the server dispatches its
// own providers.
pub enum CloudflareApi {
    Live(CloudflareClient),
    Recording(Arc<RecordingApi>),
}

impl CloudflareApi {
    pub async fn create(&self, kind: &str, fqdn: &str, content: &str) -> Result<String, DnsError> {
        match self {
            Self::Live(client) => client.create(kind, fqdn, content).await,
            Self::Recording(recording) => Ok(recording.create(kind, fqdn, content)),
        }
    }

    pub async fn delete(&self, record_id: &str) -> Result<(), DnsError> {
        match self {
            Self::Live(client) => client.delete(record_id).await,
            Self::Recording(recording) => {
                recording.delete(record_id);
                Ok(())
            }
        }
    }
}
