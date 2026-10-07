use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Context, bail};
use serde_json::{Value, json};

use super::PackIdentity;
use crate::fs_util::AtomicWrite;

/// A world's `world_behavior_packs.json` or `world_resource_packs.json`: the packs BDS enables
/// for that world, each by header UUID and the exact version of the installed pack.
pub struct WorldPacksFile {
    path: PathBuf,
}

impl WorldPacksFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Lists `pack` at its installed version. An entry for any of `replaces` is replaced where it
    /// stands; other packs keep their entries and order. Returns whether the file changed.
    pub fn register(&self, pack: &PackIdentity, replaces: &[&str]) -> anyhow::Result<bool> {
        let entries = self.read()?;
        let wanted = json!({ "pack_id": pack.uuid, "version": pack.version });

        let mut next = Vec::with_capacity(entries.len() + 1);
        let mut placed = false;
        for entry in &entries {
            let id = entry.get("pack_id").and_then(Value::as_str).unwrap_or_default();
            let is_bvc = id == pack.uuid || replaces.contains(&id);
            if !is_bvc {
                next.push(entry.clone());
            } else if !placed {
                next.push(wanted.clone());
                placed = true;
            }
        }
        if !placed {
            next.push(wanted);
        }

        if next == entries {
            return Ok(false);
        }
        let mut bytes = serde_json::to_vec_pretty(&next)?;
        bytes.push(b'\n');
        AtomicWrite::write(&self.path, &bytes)?;
        Ok(true)
    }

    // A missing file is an empty list. Anything that is not a JSON array is an error, so a
    // broken file is reported and never replaced.
    fn read(&self) -> anyhow::Result<Vec<Value>> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", self.path.display())),
        };
        match serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", self.path.display()))?
        {
            Value::Array(entries) => Ok(entries),
            _ => bail!("{} is not a JSON array", self.path.display()),
        }
    }
}
