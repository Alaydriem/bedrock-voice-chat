use serde_json::json;

use crate::config::SwarmConfig;
use crate::error_chain::ErrorChain;

/// Provisions bot players and single-use login codes through the admin API.
/// Holds a reqwest client bound to the admin mTLS identity; the server rejects
/// these routes without an admin client cert.
pub struct CodeMinter {
    client: reqwest::Client,
    server: String,
}

impl CodeMinter {
    pub fn new(config: &SwarmConfig) -> Result<Self, anyhow::Error> {
        Self::from_paths(
            &config.server,
            config.ca.as_deref(),
            &config.admin_cert,
            &config.admin_key,
        )
    }

    /// The same minter for a caller that holds the four values without a `SwarmConfig`
    /// around them.
    pub fn from_paths(
        server: &str,
        ca: Option<&str>,
        admin_cert: &str,
        admin_key: &str,
    ) -> Result<Self, anyhow::Error> {
        let cert = std::fs::read(admin_cert)
            .map_err(|e| anyhow::anyhow!("reading admin_cert {}: {}", admin_cert, e))?;
        let key = std::fs::read(admin_key)
            .map_err(|e| anyhow::anyhow!("reading admin_key {}: {}", admin_key, e))?;

        // reqwest::Identity::from_pem wants the cert and key concatenated in one PEM.
        let mut identity_pem = cert.clone();
        identity_pem.extend_from_slice(b"\n");
        identity_pem.extend_from_slice(&key);
        let identity = reqwest::Identity::from_pem(&identity_pem)
            .map_err(|e| anyhow::anyhow!("building admin identity: {}", e))?;

        let mut builder = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(identity);

        if let Some(ca_path) = ca {
            let ca_bytes = std::fs::read(ca_path)
                .map_err(|e| anyhow::anyhow!("reading ca {}: {}", ca_path, e))?;
            let ca_cert = reqwest::Certificate::from_pem(&ca_bytes)
                .map_err(|e| anyhow::anyhow!("parsing ca {}: {}", ca_path, e))?;
            builder = builder.add_root_certificate(ca_cert);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("building mint client: {}", e))?;

        Ok(Self {
            client,
            server: server.trim_end_matches('/').to_string(),
        })
    }

    /// Create the player if absent (201 fresh, 409 already exists — both fine).
    async fn ensure_player(&self, gamertag: &str) -> Result<(), anyhow::Error> {
        let resp = self
            .client
            .post(format!("{}/api/admin/user", self.server))
            .json(&json!({ "gamertag": gamertag, "game": "minecraft" }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("create user {}: {}", gamertag, ErrorChain::of(&e)))?;

        let status = resp.status().as_u16();
        if status == 201 || status == 409 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "create user {} returned {} (expected 201/409)",
                gamertag,
                status
            ))
        }
    }

    /// Mint one login code for an existing player.
    async fn mint_code(&self, gamertag: &str, duration_secs: u64) -> Result<String, anyhow::Error> {
        let resp = self
            .client
            .post(format!("{}/api/admin/user/code", self.server))
            .json(&json!({
                "gamertag": gamertag,
                "game": "minecraft",
                "duration": duration_secs,
                "ephemeral": true,
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("mint code {}: {}", gamertag, ErrorChain::of(&e)))?;

        let status = resp.status().as_u16();
        if status != 200 {
            return Err(anyhow::anyhow!(
                "mint code {} returned {} (expected 200)",
                gamertag,
                status
            ));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("mint code {} body: {}", gamertag, e))?;
        body["code"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("mint code {}: no `code` in response", gamertag))
    }

    /// Ensure each named player exists and return `(gamertag, code)` for all.
    pub async fn mint(
        &self,
        names: &[String],
        duration_secs: u64,
    ) -> Result<Vec<(String, String)>, anyhow::Error> {
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            self.ensure_player(name).await?;
            let code = self.mint_code(name, duration_secs).await?;
            out.push((name.clone(), code));
        }
        Ok(out)
    }
}
