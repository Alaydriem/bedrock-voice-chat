use std::io::{Cursor, Read};

use anyhow::{Context, anyhow};
use serde_json::Value;
use zip::ZipArchive;

/// A pack's header UUID and version, as a world's `world_*_packs.json` must name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackIdentity {
    pub uuid: String,
    pub version: Vec<u64>,
}

impl PackIdentity {
    /// Reads the identity from the `manifest.json` at the root of a `.mcpack`.
    pub fn from_mcpack(mcpack: &[u8]) -> anyhow::Result<Self> {
        let mut archive = ZipArchive::new(Cursor::new(mcpack)).context("reading the pack")?;
        let mut manifest = String::new();
        archive
            .by_name("manifest.json")
            .context("the pack has no manifest.json")?
            .read_to_string(&mut manifest)?;
        let manifest: Value = serde_json::from_str(&manifest).context("parsing manifest.json")?;

        let header = &manifest["header"];
        let uuid = header["uuid"]
            .as_str()
            .ok_or_else(|| anyhow!("manifest.json has no header.uuid"))?
            .to_string();
        let version = header["version"]
            .as_array()
            .and_then(|parts| parts.iter().map(Value::as_u64).collect::<Option<Vec<u64>>>())
            .filter(|parts| parts.len() == 3)
            .ok_or_else(|| anyhow!("manifest.json header.version is not three numbers"))?;
        Ok(Self { uuid, version })
    }
}
