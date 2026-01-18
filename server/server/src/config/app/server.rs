use serde::{Deserialize, Serialize};

use super::minecraft::ApplicationConfigMinecraft;
use super::tls::ApplicationConfigServerTLS;

fn default_listen() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u32 {
    8444
}

fn default_quic_port() -> u32 {
    8443
}

fn default_public_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_assets_path() -> String {
    "./assets".to_string()
}

/// Common server configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServer {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_http_port")]
    pub port: u32,
    #[serde(default = "default_quic_port")]
    pub quic_port: u32,
    #[serde(default = "default_public_addr")]
    pub public_addr: String,
    #[serde(default = "default_assets_path")]
    pub assets_path: String,
    #[serde(default)]
    pub tls: ApplicationConfigServerTLS,
    pub minecraft: ApplicationConfigMinecraft,
}

impl Default for ApplicationConfigServer {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            port: default_http_port(),
            quic_port: default_quic_port(),
            public_addr: default_public_addr(),
            assets_path: default_assets_path(),
            tls: ApplicationConfigServerTLS::default(),
            minecraft: ApplicationConfigMinecraft::default(),
        }
    }
}
