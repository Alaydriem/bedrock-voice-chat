use std::fs;
use std::time::Duration;

use anyhow::{Context, anyhow};
use reqwest::blocking::Client;

use super::{ReleaseAssets, ReleaseSources, UpdaterManifest};
use crate::Console;
use crate::LauncherPaths;
use crate::fs_util::AtomicWrite;

/// Downloads the newest BVC server and BDS pack when the installed copy is older.
pub struct ReleaseDownloader {
    client: Client,
    paths: LauncherPaths,
    sources: ReleaseSources,
}

impl ReleaseDownloader {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
    const ASSET_TIMEOUT: Duration = Duration::from_secs(300);

    pub fn new(paths: LauncherPaths, sources: ReleaseSources) -> anyhow::Result<Self> {
        let client = Client::builder()
            .connect_timeout(Self::CONNECT_TIMEOUT)
            .timeout(Self::ASSET_TIMEOUT)
            .build()?;
        Ok(Self {
            client,
            paths,
            sources,
        })
    }

    /// Both assets are downloaded before either replaces the installed copy, so a failed download
    /// leaves the installed pair as it was. `VERSION` is written last.
    pub fn update(&self) -> anyhow::Result<()> {
        let manifests = self
            .sources
            .manifest_urls
            .iter()
            .filter_map(|url| self.fetch_manifest(url).ok())
            .collect();
        let manifest = UpdaterManifest::newest(manifests)
            .ok_or_else(|| anyhow!("no updater manifest could be read"))?;
        let tag = manifest
            .tag()
            .ok_or_else(|| anyhow!("the updater manifest names no release"))?;

        if self.installed_tag().as_deref() == Some(tag.as_str()) {
            Console::info(&format!("BVC {tag} is up to date"));
            return Ok(());
        }

        let assets = ReleaseAssets::new(self.sources.asset_base.clone(), tag);
        let server_url = assets
            .server_url(std::env::consts::ARCH)
            .ok_or_else(|| anyhow!("no BVC server build for {}", std::env::consts::ARCH))?;

        Console::info(&format!("Downloading BVC {}", assets.tag()));
        let server = self.download(&server_url)?;
        let pack = self.download(&assets.pack_url())?;
        self.install(&server, &pack)?;
        AtomicWrite::write(&self.paths.version_file(), assets.tag().as_bytes())?;
        Ok(())
    }

    fn install(&self, server: &[u8], pack: &[u8]) -> anyhow::Result<()> {
        let server_path = self.paths.bvc_binary();
        let pack_path = self.paths.pack_file();

        let staged_server = AtomicWrite::stage(&server_path, server)?;
        if let Err(e) = AtomicWrite::mark_executable(&staged_server) {
            AtomicWrite::discard(&staged_server);
            return Err(e);
        }
        let staged_pack = match AtomicWrite::stage(&pack_path, pack) {
            Ok(staged) => staged,
            Err(e) => {
                AtomicWrite::discard(&staged_server);
                return Err(e);
            }
        };

        AtomicWrite::commit(&staged_server, &server_path)?;
        AtomicWrite::commit(&staged_pack, &pack_path)
    }

    fn fetch_manifest(&self, url: &str) -> anyhow::Result<UpdaterManifest> {
        let body = self
            .client
            .get(url)
            .timeout(self.sources.manifest_timeout)
            .send()?
            .error_for_status()?
            .text()?;
        UpdaterManifest::parse(&body)
    }

    fn installed_tag(&self) -> Option<String> {
        fs::read_to_string(self.paths.version_file())
            .ok()
            .map(|tag| tag.trim().to_string())
    }

    fn download(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let bytes = self
            .client
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.bytes())
            .with_context(|| format!("downloading {url}"))?;
        Ok(bytes.to_vec())
    }
}
