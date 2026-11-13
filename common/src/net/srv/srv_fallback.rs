#[cfg(any(target_os = "android", target_os = "ios"))]
use super::PlayZoneDns;

/// How a name is looked up when the authoritative servers do not settle it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SrvFallback {
    // The operating system's resolver configuration.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    System,
    // DNS over HTTPS (RFC 8484) to this URL.
    Https(String),
    // No fallback.
    None,
}

impl SrvFallback {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub fn platform_default() -> Self {
        Self::System
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    pub fn platform_default() -> Self {
        Self::Https(PlayZoneDns::DOH_URL.to_string())
    }
}
