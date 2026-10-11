use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use zip::ZipArchive;

use super::{InstalledPacks, PackIdentity};
use crate::fs_util::AtomicWrite;

/// Copies the BVC packs out of a `.mcaddon` into the BDS development pack directories. BDS loads
/// `.mcpack` files there as they are.
pub struct PackInstaller {
    behavior_dir: PathBuf,
    resource_dir: PathBuf,
}

impl PackInstaller {
    const BEHAVIOR_STEM: &'static str = "bedrock-voice-chat-bp";
    const RESOURCE_STEM: &'static str = "bedrock-voice-chat-rp";

    pub fn new(behavior_dir: PathBuf, resource_dir: PathBuf) -> Self {
        Self {
            behavior_dir,
            resource_dir,
        }
    }

    /// Both packs are read, and their manifests checked, before anything is written, so an
    /// incomplete add-on changes nothing. Returns the identity of each installed pack.
    pub fn install(&self, mcaddon: &Path) -> anyhow::Result<InstalledPacks> {
        let file =
            File::open(mcaddon).with_context(|| format!("opening {}", mcaddon.display()))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("reading {}", mcaddon.display()))?;

        let behavior_name = format!("{}.mcpack", Self::BEHAVIOR_STEM);
        let resource_name = format!("{}.mcpack", Self::RESOURCE_STEM);
        let behavior = Self::read_entry(&mut archive, &behavior_name)?;
        let resource = Self::read_entry(&mut archive, &resource_name)?;
        let installed = InstalledPacks {
            behavior: PackIdentity::from_mcpack(&behavior)
                .with_context(|| format!("reading {behavior_name}"))?,
            resource: PackIdentity::from_mcpack(&resource)
                .with_context(|| format!("reading {resource_name}"))?,
        };

        Self::replace(&self.behavior_dir, Self::BEHAVIOR_STEM, &behavior_name, &behavior)?;
        Self::replace(&self.resource_dir, Self::RESOURCE_STEM, &resource_name, &resource)?;
        Ok(installed)
    }

    fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> anyhow::Result<Vec<u8>> {
        let mut entry = archive
            .by_name(name)
            .with_context(|| format!("the add-on has no {name}"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    // Removes every earlier BVC pack (any file or directory named with `stem`), then writes the
    // new one.
    fn replace(dir: &Path, stem: &str, file_name: &str, bytes: &[u8]) -> anyhow::Result<()> {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !name.starts_with(stem) || name == file_name {
                continue;
            }
            if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            }
            .with_context(|| format!("removing {}", path.display()))?;
        }
        AtomicWrite::write(&dir.join(file_name), bytes)
    }
}
