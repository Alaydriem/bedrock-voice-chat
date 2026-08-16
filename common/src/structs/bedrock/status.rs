use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct BedrockStatus {
    pub proxy_running: bool,
    pub realms_running: bool,
    pub xbox_authenticated: bool,
    /// The stored credential was rejected. The app should be showing a sign-in prompt.
    pub reauth_required: bool,
    pub proxy_target_host: Option<String>,
    pub proxy_target_port: Option<u16>,
    pub proxy_listen_port: Option<u16>,
    // When the proxy came up, in unix seconds, so the UI can count for itself. A
    // duration computed here would be a snapshot that starts ageing the moment it is
    // serialised, and the status is polled.
    pub proxy_started_at: Option<u64>,
    pub active_realm_id: Option<u64>,
    pub active_realm_name: Option<String>,
    /// The name of the world this session is forwarding to, as the reader chose it in BVC
    /// Connect — a resolved saved-or-advertised entry, or the host they typed.
    ///
    /// Worth carrying separately from the host: most worlds report a level name that identifies
    /// nothing ("world", or a uuid from BDS, which cannot read one), so this is the only label a
    /// person recognises. The app remembers it against the world's id.
    pub active_connection_name: Option<String>,
}
