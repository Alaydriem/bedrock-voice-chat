use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};

use super::expiry::CertificateExpiry;

const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
const ACCOUNT_FILE: &str = "account.json";

/// Filesystem persistence for ACME material under `<certs_path>/acme/`.
pub struct AcmeStorage {
    dir: PathBuf,
}

impl AcmeStorage {
    pub fn new(certs_path: &str) -> Self {
        Self {
            dir: PathBuf::from(certs_path).join("acme"),
        }
    }

    pub fn certificate_path(&self) -> PathBuf {
        self.dir.join(CERT_FILE)
    }

    pub fn key_path(&self) -> PathBuf {
        self.dir.join(KEY_FILE)
    }

    pub fn account_path(&self) -> PathBuf {
        self.dir.join(ACCOUNT_FILE)
    }

    /// The stored certificate, but only while it stays valid for at least
    /// `min_validity` — an expiring cert is treated as absent so the caller
    /// re-issues.
    pub fn load_certificate_valid_for(&self, min_validity: Duration) -> Result<Option<String>> {
        let path = self.certificate_path();
        if !path.exists() || !self.key_path().exists() {
            return Ok(None);
        }
        let pem = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        match CertificateExpiry::is_valid_for(&pem, min_validity) {
            Ok(true) => Ok(Some(pem)),
            Ok(false) => Ok(None),
            // An unparseable stored cert means re-issue, not crash.
            Err(_) => Ok(None),
        }
    }

    pub fn store_certificate(&self, cert_pem: &str, key_pem: &str) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        fs::write(self.certificate_path(), cert_pem)
            .with_context(|| format!("writing {}", self.certificate_path().display()))?;
        Self::write_restricted(&self.key_path(), key_pem)
    }

    pub fn load_account_credentials(&self) -> Result<Option<String>> {
        let path = self.account_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(fs::read_to_string(&path).with_context(|| {
            format!("reading {}", path.display())
        })?))
    }

    pub fn store_account_credentials(&self, json: &str) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        Self::write_restricted(&self.account_path(), json)
    }

    /// Owner-only permissions on Unix; Windows ACLs inherit from the certs
    /// directory (same contract as ca.key and the access token file). Unlike
    /// the access token's create_new, renewal overwrites in place.
    fn write_restricted(path: &Path, content: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)
                .with_context(|| format!("creating {}", path.display()))?;
            file.write_all(content.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
        }
        #[cfg(not(unix))]
        {
            fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(())
    }
}
