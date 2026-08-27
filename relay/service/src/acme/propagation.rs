use std::time::Duration;

use serde_json::Value;

use super::error::AcmeError;

// Confirms the challenge TXT is publicly visible before the certificate authority is
// asked to look.
//
// A failed validation invalidates the whole order, and orders are the scarce thing
// here — checking first costs a few seconds and saves an issuance from the budget.
// DNS-over-HTTPS so no DNS library is needed, and it follows CNAMEs.
pub struct PropagationCheck {
    http: reqwest::Client,
    doh_url: String,
    interval: Duration,
    timeout: Duration,
}

impl PropagationCheck {
    const DEFAULT_DOH: &'static str = "https://cloudflare-dns.com/dns-query";

    const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

    pub fn new() -> Self {
        Self::new_with(Self::DEFAULT_DOH, Self::DEFAULT_INTERVAL, Self::DEFAULT_TIMEOUT)
    }

    pub fn new_with(doh_url: &str, interval: Duration, timeout: Duration) -> Self {
        Self {
            http: reqwest::Client::new(),
            doh_url: doh_url.to_string(),
            interval,
            timeout,
        }
    }

    pub async fn wait_for(&self, fqdn: &str, expected: &str) -> Result<(), AcmeError> {
        let deadline = tokio::time::Instant::now() + self.timeout;

        loop {
            if self.visible(fqdn, expected).await {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(AcmeError::Propagation(fqdn.to_string()));
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    async fn visible(&self, fqdn: &str, expected: &str) -> bool {
        let Ok(response) = self
            .http
            .get(&self.doh_url)
            .query(&[("name", fqdn), ("type", "TXT")])
            .header("accept", "application/dns-json")
            .send()
            .await
        else {
            return false;
        };

        let Ok(body) = response.json::<Value>().await else {
            return false;
        };

        body["Answer"]
            .as_array()
            .map(|answers| {
                answers.iter().any(|answer| {
                    answer["data"]
                        .as_str()
                        .map(|data| data.trim_matches('"') == expected)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
}

impl Default for PropagationCheck {
    fn default() -> Self {
        Self::new()
    }
}
