use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::AddonTransport;
use serde::{Deserialize, Serialize};

fn default_port() -> u16 {
    19132
}

// One operator-curated Bedrock server from the `bedrock { servers = [...] }`
// config list, advertised to clients through `/api/config`.
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct BedrockServerEntry {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    // Raw Bedrock protocol version the client proxy should advertise for this
    // server. None means Auto — mirror the real backend's version.
    #[serde(default)]
    pub protocol_version: Option<u32>,
    // How this world's addon reaches the BVC server. Declared by the operator
    // because the client cannot observe it.
    #[serde(default)]
    pub addon_transport: AddonTransport,
}

impl BedrockServerEntry {
    // Parses the compact env-var form `Name@host[:port][@protocol][@transport]`,
    // e.g. `The Hive@geo.hivebedrock.network` or
    // `Custom@play.example.com:25000@844@net`. The two trailing tokens are
    // classified by shape rather than position, so either may be omitted and
    // their order does not matter.
    // Hostnames only — an IPv6 literal's colons would be read as a port split.
    pub fn from_compact(raw: &str) -> Result<Self, anyhow::Error> {
        let mut parts = raw.splitn(4, '@');
        let name = parts.next().map(str::trim).unwrap_or_default();
        let host_port = parts.next().map(str::trim);
        let protocol = parts.next().map(str::trim);
        let extra = parts.next().map(str::trim);

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

        let mut protocol_version = None;
        let mut addon_transport = AddonTransport::default();

        for token in [protocol, extra].into_iter().flatten() {
            if token.is_empty() {
                continue;
            }
            if token.chars().all(|c| c.is_ascii_digit()) {
                protocol_version = Some(token.parse::<u32>().map_err(|_| {
                    anyhow::anyhow!("invalid protocol version {token:?} in {raw:?}")
                })?);
                continue;
            }
            addon_transport = match token {
                "net" => AddonTransport::Net,
                "no_net" => AddonTransport::NoNet,
                other => {
                    return Err(anyhow::anyhow!(
                        "unrecognized token {other:?} in {raw:?}; \
                         expected a protocol version, `net`, or `no_net`"
                    ));
                }
            };
        }

        Ok(Self {
            name: name.to_string(),
            host,
            port,
            protocol_version,
            addon_transport,
        })
    }

    pub fn to_api(&self) -> ApiConfigBedrockServer {
        ApiConfigBedrockServer {
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            protocol_version: self.protocol_version,
            addon_transport: self.addon_transport,
        }
    }
}
