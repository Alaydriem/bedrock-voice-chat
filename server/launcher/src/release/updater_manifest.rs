use std::collections::BTreeMap;

use anyhow::Context;
use serde::Deserialize;

use super::PlatformEntry;

/// The Tauri updater manifest `release.yml` publishes for each channel.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdaterManifest {
    pub pub_date: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, PlatformEntry>,
}

impl UpdaterManifest {
    const DOWNLOAD_SEGMENT: &'static str = "/releases/download/";

    pub fn parse(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).context("parsing updater manifest")
    }

    /// The release tag, taken from the path of any platform's download URL.
    pub fn tag(&self) -> Option<String> {
        self.platforms.values().find_map(|platform| {
            platform
                .url
                .split(Self::DOWNLOAD_SEGMENT)
                .nth(1)?
                .split('/')
                .next()
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
        })
    }

    /// Both manifests come from the same generator, so their RFC 3339 `pub_date` strings share a
    /// format and compare correctly as text.
    pub fn newest(manifests: Vec<UpdaterManifest>) -> Option<UpdaterManifest> {
        manifests
            .into_iter()
            .max_by(|a, b| a.pub_date.cmp(&b.pub_date))
    }
}
