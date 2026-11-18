use serde::{Deserialize, Serialize};

// What a mod reports about its host: a Java mod (`variant`, `platform`, `fetch`,
// `write`, plus `loader` and `mode`) says whether the host could run the skinny jar,
// and the Bedrock add-on sends the `loader` and `mode` shape without the jar fields.
//
// Two capabilities rather than one: a host that can fetch a library but cannot
// write it to disk cannot run the skinny jar either, and reachability alone would
// record that host as ready.
//
// The vocabulary is closed. This arrives from a third-party mod over a route the
// server does not control, and an unrecognised value would put an unbounded string
// into the metrics pipeline as a property name's value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    pub mod_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl HostCapability {
    const VARIANTS: [&'static str; 2] = ["fat", "skinny"];

    const PLATFORMS: [&'static str; 4] =
        ["windows-x64", "linux-x64", "linux-arm64", "darwin-arm64"];

    const FETCH: [&'static str; 6] = ["ok", "dns", "tls", "timeout", "refused", "io"];

    const WRITE: [&'static str; 5] = ["ok", "permission_denied", "no_space", "io", "skipped"];

    const LOADERS: [&'static str; 4] = ["paper", "fabric", "powernukkitx", "bds"];

    const MODES: [&'static str; 2] = ["embedded", "external"];

    // Bounded so a mod cannot report a version string long enough to matter.
    const MAX_VERSION_LEN: usize = 64;

    pub fn parse(json: &str) -> Result<Self, String> {
        let report: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        report.validate()?;
        Ok(report)
    }

    fn validate(&self) -> Result<(), String> {
        match (&self.loader, &self.mode) {
            (Some(loader), Some(mode)) => {
                if !Self::LOADERS.contains(&loader.as_str()) {
                    return Err(format!("unknown loader: {}", loader));
                }
                if !Self::MODES.contains(&mode.as_str()) {
                    return Err(format!("unknown mode: {}", mode));
                }
            }
            (None, None) => {}
            _ => return Err("loader and mode must be reported together".to_string()),
        }

        if self.loader.as_deref() == Some("bds") {
            if self.variant.is_some()
                || self.platform.is_some()
                || self.fetch.is_some()
                || self.write.is_some()
            {
                return Err("a bds report carries no jar fields".to_string());
            }
            if self.mode.as_deref() != Some("external") {
                return Err("a bds report is always external".to_string());
            }
        } else {
            let variant = Self::required(&self.variant, "variant")?;
            if !Self::VARIANTS.contains(&variant) {
                return Err(format!("unknown variant: {}", variant));
            }

            let platform = Self::required(&self.platform, "platform")?;
            if !Self::PLATFORMS.contains(&platform) {
                return Err(format!("unknown platform: {}", platform));
            }

            let fetch = Self::required(&self.fetch, "fetch")?;
            if !Self::FETCH.contains(&fetch) && !Self::is_http_outcome(fetch) {
                return Err(format!("unknown fetch outcome: {}", fetch));
            }

            let write = Self::required(&self.write, "write")?;
            if !Self::WRITE.contains(&write) {
                return Err(format!("unknown write outcome: {}", write));
            }
        }

        if self.mod_version.is_empty() || self.mod_version.len() > Self::MAX_VERSION_LEN {
            return Err(format!("implausible mod version: {}", self.mod_version));
        }

        Ok(())
    }

    fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, String> {
        value
            .as_deref()
            .ok_or_else(|| format!("missing {}", name))
    }

    // `http_<status>` cannot be a fixed list without the server needing a release
    // every time a proxy invents a status, so it is checked by shape instead.
    fn is_http_outcome(value: &str) -> bool {
        match value.strip_prefix("http_") {
            Some(status) => status.len() == 3 && status.chars().all(|c| c.is_ascii_digit()),
            None => false,
        }
    }
}
