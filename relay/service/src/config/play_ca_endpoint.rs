use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::error::ConfigError;

// Where the play CA's fixed name connects: an IP address or a hostname, and a port. A
// hostname such as a compose service name is resolved on every new connection, so a CA
// container that comes back with another address is found without a registry restart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PlayCaEndpoint {
    host: String,
    port: u16,
}

impl PlayCaEndpoint {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl FromStr for PlayCaEndpoint {
    type Err = ConfigError;

    // `host:port`, `a.b.c.d:port` or `[ipv6]:port`. An IPv6 address needs its brackets:
    // without them the port cannot be told apart from the last group.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid = || {
            ConfigError::Parse(format!(
                "play.ca.connect must be host:port, [ipv6]:port or ip:port, not {value:?}"
            ))
        };

        let (host, port) = value.rsplit_once(':').ok_or_else(invalid)?;
        let host = match host.strip_prefix('[') {
            Some(bracketed) => bracketed.strip_suffix(']').ok_or_else(invalid)?,
            None if host.contains(':') => return Err(invalid()),
            None => host,
        };
        if host.is_empty() || host.contains(char::is_whitespace) {
            return Err(invalid());
        }
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or_else(invalid)?;

        Ok(Self {
            host: host.to_string(),
            port,
        })
    }
}

impl TryFrom<String> for PlayCaEndpoint {
    type Error = ConfigError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl fmt::Display for PlayCaEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

impl From<PlayCaEndpoint> for String {
    fn from(endpoint: PlayCaEndpoint) -> Self {
        endpoint.to_string()
    }
}
