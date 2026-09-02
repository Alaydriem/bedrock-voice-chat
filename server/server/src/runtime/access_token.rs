//! Minecraft access-token resolution for the QUIC/HTTP server.
//!
//! Resolution order: configured value (env or config.hcl, already merged upstream) >
//! database > the file a pre-database deployment left behind. Nothing is generated: a
//! deployment that configures none of these has no scalar credential, and an operator
//! mints an identified token instead.

use std::path::PathBuf;

use anyhow::{Context, Result};
use common::curia;
use sea_orm::ConnectionTrait;

use super::secret_store::{SecretName, SecretStore};

const TOKEN_FILE_NAME: &str = "access_token";

/// Resolves the pre-identifier access token.
pub struct AccessTokenManager {
    legacy_path: PathBuf,
}

impl AccessTokenManager {
    pub fn new(certs_path: &str) -> Self {
        Self {
            legacy_path: PathBuf::from(certs_path).join(TOKEN_FILE_NAME),
        }
    }

    /// Returns the configured value, the stored row, or an imported pre-database file, in
    /// that order. `None` when a deployment has none of them.
    pub async fn resolve<C: ConnectionTrait>(
        &self,
        conn: &C,
        configured: &str,
    ) -> Result<Option<String>> {
        let configured = configured.trim();
        if !configured.is_empty() {
            SecretStore::write(conn, SecretName::MinecraftAccessToken, configured).await?;
            return Ok(Some(configured.to_string()));
        }

        if let Some(value) = SecretStore::read(conn, SecretName::MinecraftAccessToken).await? {
            return Ok(Some(value));
        }

        if self.legacy_path.exists() {
            let value = std::fs::read_to_string(&self.legacy_path)
                .with_context(|| format!("reading {}", self.legacy_path.display()))?
                .trim()
                .to_string();
            if !value.is_empty() {
                curia::info!(
                    "Importing an existing on-disk access token into the database. The file is no longer read and can be removed.",
                    { "path": self.legacy_path.display().to_string() }
                );
                SecretStore::write(conn, SecretName::MinecraftAccessToken, &value).await?;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }
}
