use common::response::ApiConfigBedrockServer;
use serde::{Deserialize, Serialize};

fn default_port() -> u16 {
    19132
}

// One operator-curated Bedrock server from the `bedrock { servers = [...] }`
// config list, advertised to clients through `/api/config`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BedrockServerEntry {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    // Raw Bedrock protocol version the client proxy should advertise for this
    // server. None means Auto — mirror the real backend's version.
    #[serde(default)]
    pub protocol_version: Option<u32>,
}

impl BedrockServerEntry {
    // Parses the compact env-var form `Name@host[:port][@protocol]`, e.g.
    // `The Hive@geo.hivebedrock.network` or `Custom@play.example.com:25000@844`.
    // Hostnames only — an IPv6 literal's colons would be read as a port split.
    pub fn from_compact(raw: &str) -> Result<Self, anyhow::Error> {
        let mut parts = raw.splitn(3, '@');
        let name = parts.next().map(str::trim).unwrap_or_default();
        let host_port = parts.next().map(str::trim);
        let protocol = parts.next().map(str::trim);

        let host_port = match (name.is_empty(), host_port) {
            (false, Some(hp)) if !hp.is_empty() => hp,
            _ => {
                return Err(anyhow::anyhow!(
                    "expected `Name@host[:port][@protocol]`, got {raw:?}"
                ));
            }
        };

        let (host, port) = match host_port.rsplit_once(':') {
            Some((host, port_str)) => {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|_| anyhow::anyhow!("invalid port {port_str:?} in {raw:?}"))?;
                (host.to_string(), port)
            }
            None => (host_port.to_string(), default_port()),
        };

        let protocol_version = match protocol {
            None => None,
            Some(p) => Some(
                p.parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("invalid protocol version {p:?} in {raw:?}"))?,
            ),
        };

        Ok(Self {
            name: name.to_string(),
            host,
            port,
            protocol_version,
        })
    }

    pub fn to_api(&self) -> ApiConfigBedrockServer {
        ApiConfigBedrockServer {
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            protocol_version: self.protocol_version,
        }
    }
}
