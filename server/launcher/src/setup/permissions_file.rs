use std::path::PathBuf;

use anyhow::bail;
use serde_json::Value;

use crate::fs_util::JsonFile;

/// BDS `config/default/permissions.json`: the script modules a pack may import.
pub struct PermissionsFile {
    path: PathBuf,
}

impl PermissionsFile {
    pub const REQUIRED_MODULES: [&'static str; 4] = [
        "@minecraft/server",
        "@minecraft/server-ui",
        "@minecraft/server-net",
        "@minecraft/server-admin",
    ];

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Adds each required module that is not listed. Returns whether the file changed.
    pub fn ensure_required_modules(&self) -> anyhow::Result<bool> {
        let mut root = JsonFile::read_object(&self.path)?;
        let modules = root
            .entry("allowed_modules")
            .or_insert_with(|| Value::Array(Vec::new()));
        let Value::Array(list) = modules else {
            bail!("allowed_modules in {} is not an array", self.path.display());
        };

        let missing: Vec<&str> = Self::REQUIRED_MODULES
            .into_iter()
            .filter(|module| !list.iter().any(|v| v.as_str() == Some(module)))
            .collect();
        if missing.is_empty() {
            return Ok(false);
        }

        list.extend(missing.into_iter().map(|m| Value::String(m.to_string())));
        JsonFile::write_object(&self.path, &root)?;
        Ok(true)
    }
}
