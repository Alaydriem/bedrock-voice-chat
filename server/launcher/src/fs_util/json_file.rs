use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, bail};
use serde_json::{Map, Value};

use super::AtomicWrite;

/// A JSON file whose top level is an object. Key order is kept.
pub struct JsonFile;

impl JsonFile {
    /// A missing file reads as an empty object. Invalid JSON is an error, so a broken file is
    /// reported and never replaced.
    pub fn read_object(path: &Path) -> anyhow::Result<Map<String, Value>> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Map::new()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        match serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?
        {
            Value::Object(object) => Ok(object),
            _ => bail!("{} is not a JSON object", path.display()),
        }
    }

    pub fn write_object(path: &Path, object: &Map<String, Value>) -> anyhow::Result<()> {
        let mut bytes = serde_json::to_vec_pretty(object)?;
        bytes.push(b'\n');
        AtomicWrite::write(path, &bytes)
    }
}
