use std::time::Duration;

use anyhow::{Result, anyhow};

const DEFAULT_DOH_URL: &str = "https://cloudflare-dns.com/dns-query";
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// Confirms a challenge TXT record is publicly visible before telling the CA
/// to validate — a failed validation invalidates the whole order, so probing
/// first is cheaper than retrying orders. Uses DNS-over-HTTPS (JSON) so no
/// DNS library is needed, and follows CNAMEs (which is what makes acme-dns
/// delegation work transparently).
pub struct PropagationChecker {
    doh_url: String,
    poll_interval: Duration,
    timeout: Duration,
    client: reqwest::Client,
}

impl PropagationChecker {
    pub fn new() -> Self {
        Self::new_with(DEFAULT_DOH_URL, DEFAULT_POLL_INTERVAL, DEFAULT_TIMEOUT)
    }

    pub fn new_with(doh_url: &str, poll_interval: Duration, timeout: Duration) -> Self {
        Self {
            doh_url: doh_url.to_string(),
            poll_interval,
            timeout,
            client: reqwest::Client::new(),
        }
    }

    pub async fn wait_for_txt(&self, fqdn: &str, expected_value: &str) -> Result<()> {
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            if self.txt_visible(fqdn, expected_value).await.unwrap_or(false) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(anyhow!(
                    "DNS propagation timed out: TXT {fqdn} never showed the challenge value"
                ));
            }
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn txt_visible(&self, fqdn: &str, expected_value: &str) -> Result<bool> {
        let response: serde_json::Value = self
            .client
            .get(&self.doh_url)
            .query(&[("name", fqdn), ("type", "TXT")])
            .header("accept", "application/dns-json")
            .send()
            .await?
            .json()
            .await?;
        let Some(answers) = response["Answer"].as_array() else {
            return Ok(false);
        };
        // TXT rdata arrives quoted; match on containment to tolerate both
        // quoted and split-string forms.
        Ok(answers.iter().any(|answer| {
            answer["data"]
                .as_str()
                .map(|data| data.contains(expected_value))
                .unwrap_or(false)
        }))
    }
}

impl Default for PropagationChecker {
    fn default() -> Self {
        Self::new()
    }
}
