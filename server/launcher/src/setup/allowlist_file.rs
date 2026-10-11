use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use serde_json::Value;

/// The names in BDS `allowlist.json`. Entries without a non-blank string `name` are skipped.
pub struct AllowlistFile {
    names: Vec<String>,
    skipped: usize,
}

impl AllowlistFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let Value::Array(entries) = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?
        else {
            bail!("{} is not a JSON array", path.display());
        };

        let total = entries.len();
        let names: Vec<String> = entries
            .iter()
            .filter_map(|entry| entry.get("name")?.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
        Ok(Self {
            skipped: total - names.len(),
            names,
        })
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn skipped(&self) -> usize {
        self.skipped
    }
}
