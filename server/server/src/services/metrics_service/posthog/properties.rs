use serde::Serialize;

use crate::services::metrics_service::heartbeat_snapshot::HeartbeatSnapshot;
use crate::services::metrics_service::host_capability::HostCapability;

#[derive(Debug, Clone, Serialize)]
pub struct EventProperties {
    pub server_id: String,
    pub server_version: String,
    // Blake3 of the server cert's CN. Not a migration bridge for server_id: before
    // this release it rode only on Server::Started, so no other event type ever
    // carried it and nothing historical can be joined on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_duration_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_since_disconnect_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<&'static str>,
    // Carried by Server::Stopped. HeartbeatSnapshot flattens a field of the same
    // name, so exactly one of this and `heartbeat` may ever be Some: setting both
    // emits `uptime_secs` twice in one JSON object and the duplicate resolves
    // silently at whatever parses it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
    #[serde(flatten)]
    pub heartbeat: Option<HeartbeatSnapshot>,
    // Nested rather than flattened, unlike `heartbeat`. Its fields are `variant`,
    // `platform` and `fetch` — names general enough that flattening them into the
    // shared property namespace invites exactly the silent duplicate-key collision
    // described above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_capability: Option<HostCapability>,
}
