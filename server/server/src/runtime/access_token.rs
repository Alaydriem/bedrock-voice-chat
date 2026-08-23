//! Minecraft access-token resolution for the QUIC/HTTP server.
//!
//! Resolution order: configured value (env or config.hcl, already merged upstream) >
//! database > the file a pre-database deployment left behind > newly generated. The
//! database is the durable copy, so a container needs no persistent volume for it.

use std::path::PathBuf;

use anyhow::Result;
use rand::RngExt;
use rand::distr::Alphanumeric;
use sea_orm::ConnectionTrait;

use super::secret_store::{SecretName, SecretStore};

const TOKEN_FILE_NAME: &str = "access_token";
const TOKEN_LENGTH: usize = 32;

/// Resolves and, when necessary, generates the access token.
pub struct AccessTokenManager {
    legacy_path: PathBuf,
}

impl AccessTokenManager {
    pub fn new(certs_path: &str) -> Self {
        Self {
            legacy_path: PathBuf::from(certs_path).join(TOKEN_FILE_NAME),
        }
    }

    /// Returns the effective access token, leaving the database holding it.
    pub async fn resolve<C: ConnectionTrait>(&self, conn: &C, configured: &str) -> Result<String> {
        SecretStore::resolve(
            conn,
            SecretName::MinecraftAccessToken,
            Some(configured),
            Some(self.legacy_path.as_path()),
            Self::generate,
        )
        .await
    }

    fn generate() -> String {
        let mut rng = rand::rng();
        (0..TOKEN_LENGTH)
            .map(|_| char::from(rng.sample(Alphanumeric)))
            .collect()
    }
}
