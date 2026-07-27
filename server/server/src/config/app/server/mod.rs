pub mod acme;
pub mod age;
pub mod bedrock;
pub mod cors;
pub mod features;
pub mod meridian;
pub mod minecraft;
pub mod relay;
pub mod tls;

pub use acme::{Acme, AcmeProviderKind};
pub use age::Age;
pub use bedrock::BedrockConfig;
pub use bedrock::BedrockDnsConfig;
pub use bedrock::BedrockServerEntry;
pub use cors::Cors;
pub use features::Features;
pub use meridian::Meridian;
pub use minecraft::Minecraft;
pub use tls::Tls;

use serde::{Deserialize, Serialize};

fn default_listen() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u32 {
    443
}

fn default_quic_port() -> u32 {
    443
}

fn default_advertised_quic_ports() -> Vec<u32> {
    Vec::new()
}

fn default_assets_path() -> String {
    "./assets".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_http_port")]
    pub port: u32,
    #[serde(default = "default_quic_port")]
    pub quic_port: u32,
    #[serde(default = "default_advertised_quic_ports")]
    pub advertised_quic_ports: Vec<u32>,
    #[serde(default = "default_assets_path")]
    pub assets_path: String,
    #[serde(default)]
    pub tls: Tls,
    #[serde(default)]
    pub cors: Cors,
    #[serde(default)]
    pub minecraft: Minecraft,
    #[serde(default)]
    pub features: Features,
    #[serde(default)]
    pub meridian: Option<Meridian>,
    #[serde(default)]
    pub bedrock: BedrockConfig,
    #[serde(default)]
    pub age: Age,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            port: default_http_port(),
            quic_port: default_quic_port(),
            advertised_quic_ports: default_advertised_quic_ports(),
            assets_path: default_assets_path(),
            tls: Tls::default(),
            cors: Cors::default(),
            minecraft: Minecraft::default(),
            features: Features::default(),
            meridian: None,
            bedrock: BedrockConfig::default(),
            age: Age::default(),
        }
    }
}

impl Server {
    // Public UDP ports a client should try, in the operator's preferred order.
    // Deliberately independent of `quic_port`: the server binds one socket, but a
    // fronting proxy and a direct port publish can both deliver to it, so
    // reachability is many-to-one. An empty list means the bind port is the only
    // way in.
    pub fn quic_ports(&self) -> Vec<u32> {
        if self.advertised_quic_ports.is_empty() {
            return vec![self.quic_port];
        }

        self.advertised_quic_ports.clone()
    }
}
