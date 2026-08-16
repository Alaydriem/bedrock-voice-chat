pub mod identity;
pub mod metadata;
pub mod summary;

pub use identity::Identity;
pub use metadata::IdentityMetadata;
pub use summary::IdentitySummary;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, anyhow};

use crate::identity::IdentitySlot;
use crate::identity::secrets::{DefaultBackend, SecretBackend};

pub struct IdentityStore;

impl IdentityStore {
    pub fn metadata_dir() -> Result<PathBuf, anyhow::Error> {
        let base = match std::env::var("BVC_IDENTITY_DIR") {
            Ok(v) => PathBuf::from(v),
            Err(_) => dirs::home_dir()
                .ok_or_else(|| anyhow!("home directory not found"))?
                .join(".bvc")
                .join("identity"),
        };
        fs::create_dir_all(&base).context("create identity dir")?;
        Ok(base)
    }

    fn metadata_path(slot: &IdentitySlot) -> Result<PathBuf, anyhow::Error> {
        Ok(Self::metadata_dir()?.join(format!("{}.toml", slot.key())))
    }

    pub fn save(identity: &Identity) -> Result<(), anyhow::Error> {
        let slot = IdentitySlot::new(identity.gamertag.clone(), identity.game.clone());
        let backend = DefaultBackend::new();
        backend.save(
            &slot.key(),
            &identity.cert_pem,
            &identity.key_pem,
            &identity.ca_pem,
        )?;

        let metadata = IdentityMetadata {
            gamertag: identity.gamertag.clone(),
            game: identity.game.clone(),
            server_url: identity.server_url.clone(),
            cert_not_after: identity.cert_not_after,
        };
        let path = Self::metadata_path(&slot)?;
        let toml_text = toml::to_string_pretty(&metadata).context("serialize metadata")?;
        fs::write(&path, toml_text).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn load(slot: &IdentitySlot) -> Result<Identity, anyhow::Error> {
        let path = Self::metadata_path(slot)?;
        let toml_text = fs::read_to_string(&path)
            .with_context(|| format!("read {} (run `bvc login` first)", path.display()))?;
        let metadata: IdentityMetadata =
            toml::from_str(&toml_text).context("parse identity metadata")?;

        let backend = DefaultBackend::new();
        let (cert_pem, key_pem, ca_pem) = backend.load(&slot.key())?;

        Ok(Identity {
            gamertag: metadata.gamertag,
            game: metadata.game,
            server_url: metadata.server_url,
            cert_pem,
            key_pem,
            ca_pem,
            cert_not_after: metadata.cert_not_after,
        })
    }

    pub fn delete(slot: &IdentitySlot) -> Result<(), anyhow::Error> {
        let backend = DefaultBackend::new();
        backend.delete(&slot.key())?;
        let path = Self::metadata_path(slot)?;
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
        }
        Ok(())
    }

    pub fn list() -> Result<Vec<IdentitySummary>, anyhow::Error> {
        let dir = Self::metadata_dir()?;
        let mut out = Vec::new();
        for entry in fs::read_dir(&dir).with_context(|| format!("read_dir {}", dir.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let toml_text = fs::read_to_string(&path)?;
            let metadata: IdentityMetadata = match toml::from_str(&toml_text) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("skipping bad identity {}: {}", path.display(), e);
                    continue;
                }
            };
            out.push(IdentitySummary {
                slot: IdentitySlot::new(metadata.gamertag.clone(), metadata.game.clone()),
                metadata,
            });
        }
        Ok(out)
    }
}
