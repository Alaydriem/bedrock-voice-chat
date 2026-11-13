use super::{PlayZoneDns, SrvFallback};

/// Where a [`super::SrvResolver`] asks.
#[derive(Clone, Debug)]
pub struct SrvResolverConfig {
    // `host:port` of each authoritative nameserver for the play zone, asked in order.
    pub nameservers: Vec<String>,
    pub fallback: SrvFallback,
}

impl SrvResolverConfig {
    pub fn production() -> Self {
        Self {
            nameservers: Self::nameservers(),
            fallback: SrvFallback::platform_default(),
        }
    }

    #[cfg(all(debug_assertions, feature = "dev-trust"))]
    fn nameservers() -> Vec<String> {
        super::DevPlayNameservers::from_env().unwrap_or_else(Self::play_zone_nameservers)
    }

    #[cfg(not(all(debug_assertions, feature = "dev-trust")))]
    fn nameservers() -> Vec<String> {
        Self::play_zone_nameservers()
    }

    fn play_zone_nameservers() -> Vec<String> {
        PlayZoneDns::NAMESERVERS
            .iter()
            .map(|nameserver| nameserver.to_string())
            .collect()
    }
}
