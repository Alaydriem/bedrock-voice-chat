use serde::Deserialize;

use super::TlsSection;

fn default_port() -> u32 {
    443
}

#[derive(Debug, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_port")]
    pub port: u32,
    #[serde(default)]
    pub tls: TlsSection,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            port: default_port(),
            tls: TlsSection::default(),
        }
    }
}
