use serde::{Deserialize, Serialize};

// What a Java mod reports about whether its host could run the skinny jar.
//
// Two capabilities rather than one: a host that can fetch a library but cannot
// write it to disk cannot run the skinny jar either, and reachability alone would
// record that host as ready.
//
// The vocabulary is closed. This arrives from a third-party jar over a route the
// server does not control, and an unrecognised value would put an unbounded string
// into the metrics pipeline as a property name's value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapability {
    pub variant: String,
    pub platform: String,
    pub mod_version: String,
    pub fetch: String,
    pub write: String,
}

impl HostCapability {
    const VARIANTS: [&'static str; 2] = ["fat", "skinny"];

    const PLATFORMS: [&'static str; 4] =
        ["windows-x64", "linux-x64", "linux-arm64", "darwin-arm64"];

    const FETCH: [&'static str; 6] = ["ok", "dns", "tls", "timeout", "refused", "io"];

    const WRITE: [&'static str; 5] = ["ok", "permission_denied", "no_space", "io", "skipped"];

    // Bounded so a mod cannot report a version string long enough to matter.
    const MAX_VERSION_LEN: usize = 64;

    pub fn parse(json: &str) -> Result<Self, String> {
        let report: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        report.validate()?;
        Ok(report)
    }

    fn validate(&self) -> Result<(), String> {
        if !Self::VARIANTS.contains(&self.variant.as_str()) {
            return Err(format!("unknown variant: {}", self.variant));
        }

        if !Self::PLATFORMS.contains(&self.platform.as_str()) {
            return Err(format!("unknown platform: {}", self.platform));
        }

        if !Self::FETCH.contains(&self.fetch.as_str()) && !Self::is_http_outcome(&self.fetch) {
            return Err(format!("unknown fetch outcome: {}", self.fetch));
        }

        if !Self::WRITE.contains(&self.write.as_str()) {
            return Err(format!("unknown write outcome: {}", self.write));
        }

        if self.mod_version.is_empty() || self.mod_version.len() > Self::MAX_VERSION_LEN {
            return Err(format!("implausible mod version: {}", self.mod_version));
        }

        Ok(())
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
