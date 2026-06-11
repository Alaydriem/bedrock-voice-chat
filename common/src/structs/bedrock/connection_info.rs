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

    // Hive DNS override hint. Always `geo.hivebedrock.network`. Shown to
    // users who have pointed their device DNS at the BVC server.
    pub hive_dns_hostname: String,
}
