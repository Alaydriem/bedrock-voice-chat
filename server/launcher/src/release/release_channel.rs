/// The release channel a launcher follows. It is fixed when the launcher is built: CI sets
/// `BVC_CHANNEL=beta` for a version that contains `-`, the same rule `release.yml` uses to
/// choose between `beta.json` and `latest.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Beta,
}

impl ReleaseChannel {
    /// The channel this binary was built for. A build without `BVC_CHANNEL` follows stable.
    pub fn built() -> Self {
        Self::from_build(option_env!("BVC_CHANNEL"))
    }

    pub fn from_build(value: Option<&str>) -> Self {
        match value {
            Some("beta") => Self::Beta,
            _ => Self::Stable,
        }
    }

    pub fn manifest_file(self) -> &'static str {
        match self {
            Self::Stable => "latest.json",
            Self::Beta => "beta.json",
        }
    }
}
