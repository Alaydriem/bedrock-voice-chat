use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::{anyhow, Context};

use super::SecretBackend;

const SEPARATOR: &str = "\n-----BVC-PEM-SEPARATOR-----\n";

pub struct FileBackend;

impl FileBackend {
    pub fn new() -> Self {
        FileBackend
    }

    fn path(slot_key: &str) -> Result<PathBuf, anyhow::Error> {
        let dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("home directory not found"))?
            .join(".bvc")
            .join("identity");
        fs::create_dir_all(&dir).context("create identity dir")?;
        Ok(dir.join(format!("{}.pem", slot_key)))
    }
}

impl SecretBackend for FileBackend {
    fn save(&self, slot_key: &str, cert: &str, key: &str, ca: &str) -> Result<(), anyhow::Error> {
        let p = Self::path(slot_key)?;
        let body = format!("{}{}{}{}{}", cert, SEPARATOR, key, SEPARATOR, ca);
        fs::write(&p, body).with_context(|| format!("write {}", p.display()))?;
        let mut perms = fs::metadata(&p)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&p, perms).context("chmod identity file")?;
        Ok(())
    }

    fn load(&self, slot_key: &str) -> Result<(String, String, String), anyhow::Error> {
        let p = Self::path(slot_key)?;
        let body = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        let mut parts = body.split(SEPARATOR);
        let cert = parts.next().ok_or_else(|| anyhow!("missing cert"))?.to_string();
        let key = parts.next().ok_or_else(|| anyhow!("missing key"))?.to_string();
        let ca = parts.next().ok_or_else(|| anyhow!("missing ca"))?.to_string();
        Ok((cert, key, ca))
    }

    fn delete(&self, slot_key: &str) -> Result<(), anyhow::Error> {
        let p = Self::path(slot_key)?;
        if p.exists() {
            fs::remove_file(&p).with_context(|| format!("remove {}", p.display()))?;
        }
        Ok(())
    }
}
