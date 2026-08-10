use serde::Deserialize;

/// The shape `BedrockProxyManager.ts` writes to `store.json`.
///
/// Read rather than re-derived, so a scripted connect and the UI's own list cannot disagree
/// about what is saved. Only the fields a connect needs are named; the rest are ignored.
#[derive(Clone, Debug, Deserialize)]
pub struct SavedProxyEntry {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: Option<u32>,
}
