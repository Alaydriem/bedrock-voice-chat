use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether this client and a server speak the same protocol.
///
/// Separate from reachability on purpose: a server can answer perfectly and still be
/// unusable, and "nothing at that address" and "that server speaks a different
/// protocol" send someone to entirely different places. Only major and minor are
/// compared — patch releases do not change the wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtocolCompatibility {
    pub server_version: String,
    pub client_version: String,
    pub compatible: bool,
    /// True when the client is behind, which is the case an update can fix.
    pub client_too_old: bool,
}

impl ProtocolCompatibility {
    pub fn between(server_version: &str, client_version: &str) -> Self {
        let server = Self::major_minor(server_version);
        let client = Self::major_minor(client_version);

        Self {
            server_version: server_version.to_string(),
            client_version: client_version.to_string(),
            compatible: server == client,
            client_too_old: client < server,
        }
    }

    fn major_minor(version: &str) -> (u32, u32) {
        let parts: Vec<u32> = version
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
        )
    }
}
