/// The release asset URLs for one release tag under a download base.
pub struct ReleaseAssets {
    base: String,
    tag: String,
}

impl ReleaseAssets {
    pub fn new(base: String, tag: String) -> Self {
        Self { base, tag }
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// `arch` is a `std::env::consts::ARCH` value.
    pub fn server_url(&self, arch: &str) -> Option<String> {
        let suffix = match arch {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            _ => return None,
        };
        Some(format!("{}/{}/bvc-server-linux-{suffix}", self.base, self.tag))
    }

    /// The pack from the mods release of the same version, so server and pack always match.
    pub fn pack_url(&self) -> String {
        let version = self.tag.trim_start_matches('v');
        format!(
            "{}/mods-{}/bvc-bds-pack-{version}.mcaddon",
            self.base, self.tag
        )
    }
}
