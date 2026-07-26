//! Minecraft access-token resolution for the QUIC/HTTP server.
//!
//! Resolution order: configured value (env or config.hcl, already merged
//! upstream) > persisted file > newly generated. Generation happens exactly
//! once per certs_path — the same generate-once contract as `ca.key` — so
//! mod configurations survive server restarts.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rand::RngExt;
use rand::distr::Alphanumeric;
use tracing::info;

const TOKEN_FILE_NAME: &str = "access_token";
const TOKEN_LENGTH: usize = 32;

/// Resolves and, when necessary, generates + persists the access token.
pub struct AccessTokenManager {
    certs_path: String,
}

impl AccessTokenManager {
    pub fn new(certs_path: &str) -> Self {
        Self {
            certs_path: certs_path.to_string(),
        }
    }

    /// Returns the effective access token. A non-blank `configured` value is
    /// authoritative and never persisted; otherwise the persisted file is
    /// reused, and only if neither exists is a fresh token generated,
    /// persisted, and logged once.
    pub fn resolve(&self, configured: &str) -> Result<String> {
        let configured = configured.trim();
        if !configured.is_empty() {
            return Ok(configured.to_string());
        }

        let path = self.token_path();
        if path.exists() {
            let token = fs::read_to_string(&path)
                .with_context(|| format!("reading access token at {}", path.display()))?
                .trim()
                .to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }

        let token = Self::generate();
        fs::create_dir_all(&self.certs_path)
            .with_context(|| format!("creating certs dir {}", self.certs_path))?;
        Self::write_restricted(&path, &token)?;
        info!(
            "Generated Minecraft access token (persisted to {}): {}",
            path.display(),
            token
        );
        Ok(token)
    }

    fn token_path(&self) -> PathBuf {
        PathBuf::from(&self.certs_path).join(TOKEN_FILE_NAME)
    }

    fn generate() -> String {
        let mut rng = rand::rng();
        (0..TOKEN_LENGTH)
            .map(|_| char::from(rng.sample(Alphanumeric)))
            .collect()
    }

    /// Owner-only permissions on Unix; Windows ACLs inherit from the certs
    /// directory, matching how ca.key is already handled.
    fn write_restricted(path: &Path, token: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .with_context(|| format!("creating {}", path.display()))?;
            file.write_all(token.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
        }
        #[cfg(not(unix))]
        {
            fs::write(path, token).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(())
    }
}
