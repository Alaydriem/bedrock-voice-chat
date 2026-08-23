//! Database-backed storage for the relay node's secret key.
//!
//! Resolved on every boot, whether or not peering is configured. An operator who enables
//! peering later would otherwise find the key already gone with the volume, and every
//! far-side `peer` block naming it dead.

mod error;

pub use error::NodeKeyError;

use common::curia;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rand::RngExt;
use sea_orm::ConnectionTrait;

use super::secret_store::{SecretName, SecretStore};

const NODE_KEY_FILE_NAME: &str = "node.key";
const KEY_LEN: usize = 32;

pub struct NodeKeyStore {
    legacy_path: PathBuf,
}

impl NodeKeyStore {
    pub fn new(certs_path: &str) -> Self {
        Self {
            legacy_path: PathBuf::from(certs_path).join(NODE_KEY_FILE_NAME),
        }
    }

    /// The node's secret key, leaving the database holding it.
    ///
    /// The stored row outranks the file: this code never writes `node.key`, so a file that
    /// disagrees with the database is one an earlier release left behind.
    pub async fn resolve<C: ConnectionTrait>(&self, conn: &C) -> Result<[u8; KEY_LEN]> {
        if let Some(stored) = SecretStore::read(conn, SecretName::RelayNodeKey).await? {
            return Self::decode(&stored);
        }

        let value = SecretStore::resolve(
            conn,
            SecretName::RelayNodeKey,
            self.legacy_hex()?.as_deref(),
            None,
            Self::generate,
        )
        .await?;

        Self::decode(&value)
    }

    /// The pre-database `node.key` as hex.
    ///
    /// Read here rather than handed to `SecretStore` as a legacy path because the file holds
    /// 32 raw bytes, not text. Reached only when the database has no row, so passing it as
    /// the configured value cannot override a stored key.
    ///
    /// A file of the wrong length is an error: that is a corrupted identity, not an absent
    /// one, and generating a replacement would silently revoke this node.
    fn legacy_hex(&self) -> Result<Option<String>> {
        if !self.legacy_path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&self.legacy_path)
            .with_context(|| format!("reading {}", self.legacy_path.display()))?;
        if bytes.len() != KEY_LEN {
            return Err(NodeKeyError::WrongLength(bytes.len()).into());
        }
        curia::info!("Importing the existing relay node key into the database. The peer identity is \
             unchanged, so every peer block naming it stays valid.", { "path": self.legacy_path.display().to_string() });
        Ok(Some(hex::encode(bytes)))
    }

    fn decode(value: &str) -> Result<[u8; KEY_LEN]> {
        let bytes = hex::decode(value).map_err(|_| NodeKeyError::NotHex)?;
        let len = bytes.len();
        bytes
            .try_into()
            .map_err(|_| NodeKeyError::WrongLength(len).into())
    }

    fn generate() -> String {
        let bytes: [u8; KEY_LEN] = rand::rng().random();
        hex::encode(bytes)
    }
}
