use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::bedrock::backend_kind::BedrockBackendKind;

pub const HIVE_DNS_HOSTNAME: &str = "geo.hivebedrock.network";

// Connection info shown to the user immediately after BVC's proxy or realm
// session starts. The modal it drives tells the user exactly which address
// to type into the Minecraft "Add Server" screen on desktop and mobile.
//
// Emitted as the Tauri event `bedrock_connection_info` from
// `bedrock_start_proxy` / `bedrock_start_realms` after `start().await`
// succeeds.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct BedrockConnectionInfo {
    // Loopback address Minecraft uses when running on the same device as
    // BVC (e.g. desktop player using Bedrock for Windows).
    pub local_address: String,

    // LAN-visible address Minecraft uses from another device on the same
    // network (e.g. a phone running Bedrock connecting to BVC on a PC).
    // Resolved from the `network_interface` parameter the user picked when
    // starting the proxy / realm.
    pub lan_address: String,

    // Listen port shared by both addresses. Same value the user picked when
    // starting the proxy (or BEDROCK_LISTEN_PORT for realms).
    pub port: u16,

    // Whether BVC is forwarding to a direct Bedrock server or a Realm.
    pub backend: BedrockBackendKind,

    // Human-readable label for the upstream the user is proxying to.
    // - Direct: `"<target_host>:<target_port>"`
    // - Realm:  the realm display name picked in the UI
    pub remote_label: String,

    // Hive DNS override hint. Always `geo.hivebedrock.network`. Only meaningful
    // when the connected server runs its DNS override service; gated in the UI
    // by `server_dns_enabled`.
    pub hive_dns_hostname: String,

    // Whether the connected BVC server runs its Bedrock DNS override service
    // (`bedrock.dns_enabled` from `/api/config`). When false the UI hides the
    // Hive DNS connection option.
    #[serde(default)]
    pub server_dns_enabled: bool,

    // Transfer relay of the connected BVC server, preformatted as `host:port`.
    // The host is the server the client is connected to; the port is the
    // server's bedrock transfer port from `/api/config`. Present only when that
    // server runs the relay. Distinct from `port` (the local proxy listen port).
    #[serde(default)]
    pub server_transfer_relay: Option<String>,
}
