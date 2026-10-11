use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::fs_util::JsonFile;

/// BDS `config/default/variables.json`, read by the add-on through `@minecraft/server-admin`.
pub struct VariablesFile {
    path: PathBuf,
    values: Map<String, Value>,
}

impl VariablesFile {
    pub const SERVER_KEY: &'static str = "bvc_server";
    pub const TOKEN_KEY: &'static str = "bvc_access_token";

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            values: JsonFile::read_object(path)?,
        })
    }

    /// Absent, null and blank strings count as missing. Any other value is the operator's.
    pub fn is_missing(&self, key: &str) -> bool {
        match self.values.get(key) {
            None | Some(Value::Null) => true,
            Some(Value::String(value)) => value.trim().is_empty(),
            Some(_) => false,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.values
            .insert(key.to_string(), Value::String(value.to_string()));
    }

    pub fn save(&self) -> anyhow::Result<()> {
        JsonFile::write_object(&self.path, &self.values)
    }
}
