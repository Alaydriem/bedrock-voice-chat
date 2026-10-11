use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow, bail};
use zuri_nbt::encoding::LittleEndian;
use zuri_nbt::tag::{Byte, Compound};
use zuri_nbt::{NBTRoot, NBTTag};

use crate::fs_util::AtomicWrite;

/// Edits a Bedrock `level.dat`: an 8-byte header (storage version, payload length) followed by
/// little-endian NBT.
pub struct WorldPatcher;

impl WorldPatcher {
    const HEADER_LEN: usize = 8;
    const EXPERIMENTS: &'static str = "experiments";
    // The level.dat key behind the "Beta APIs" experiment toggle.
    const BETA_APIS: &'static str = "gametest";

    /// Sets the Beta APIs experiment. Returns whether the file changed; every other value is
    /// left as it was.
    pub fn enable_beta_apis(level_dat: &Path) -> anyhow::Result<bool> {
        let bytes =
            fs::read(level_dat).with_context(|| format!("reading {}", level_dat.display()))?;
        let (version, mut root) = Self::decode(&bytes)
            .with_context(|| format!("decoding {}", level_dat.display()))?;

        if !Self::set_beta_apis(&mut root)? {
            return Ok(false);
        }

        AtomicWrite::write(level_dat, &Self::encode(version, &root)?)?;
        Ok(true)
    }

    pub fn decode(bytes: &[u8]) -> anyhow::Result<(i32, NBTRoot)> {
        if bytes.len() < Self::HEADER_LEN {
            bail!("level.dat is shorter than its header");
        }
        let version = i32::from_le_bytes(bytes[0..4].try_into()?);
        let length = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
        let payload = bytes
            .get(Self::HEADER_LEN..Self::HEADER_LEN + length)
            .ok_or_else(|| anyhow!("level.dat payload is shorter than its header says"))?;
        let root = NBTRoot::read(payload, LittleEndian).map_err(|e| anyhow!("{e}"))?;
        Ok((version, root))
    }

    pub fn encode(version: i32, root: &NBTRoot) -> anyhow::Result<Vec<u8>> {
        let mut payload = Vec::new();
        root.write(&mut payload, LittleEndian)
            .map_err(|e| anyhow!("{e}"))?;

        let mut bytes = Vec::with_capacity(Self::HEADER_LEN + payload.len());
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(payload.len())?.to_le_bytes());
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }

    fn set_beta_apis(root: &mut NBTRoot) -> anyhow::Result<bool> {
        let NBTTag::Compound(top) = &mut root.data else {
            bail!("level.dat root is not a compound");
        };
        let experiments = top
            .0
            .entry(Self::EXPERIMENTS.to_string())
            .or_insert_with(|| NBTTag::Compound(Compound::default()));
        let NBTTag::Compound(experiments) = experiments else {
            bail!("level.dat experiments is not a compound");
        };

        if experiments.0.get(Self::BETA_APIS) == Some(&NBTTag::Byte(Byte(1))) {
            return Ok(false);
        }
        experiments
            .0
            .insert(Self::BETA_APIS.to_string(), NBTTag::Byte(Byte(1)));
        Ok(true)
    }
}
