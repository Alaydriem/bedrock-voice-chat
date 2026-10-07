use super::PackIdentity;

/// The two packs `PackInstaller` put in place.
#[derive(Debug, Clone)]
pub struct InstalledPacks {
    pub behavior: PackIdentity,
    pub resource: PackIdentity,
}
