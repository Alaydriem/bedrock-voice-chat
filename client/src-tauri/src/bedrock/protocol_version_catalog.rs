use common::bedrock_protocol::ProtocolVersion;
use common::structs::bedrock::ProtocolVersionOption;

// The advertised-version choices offered in the UI. Only protocols the
// upstream matrix reports as released are listed, so the dropdown never offers
// a version a player could not actually be running. A generated protocol can
// carry a real Minecraft version string while still being preview-only (2169 /
// "1.26.50"), so a name lookup is not sufficient to tell the two apart.
pub struct ProtocolVersionCatalog;

impl ProtocolVersionCatalog {
    pub fn released() -> Vec<ProtocolVersionOption> {
        ProtocolVersion::GENERATED_ALL
            .iter()
            .filter(|version| version.is_released())
            .map(|version| ProtocolVersionOption {
                protocol: version.0,
                label: version.client_version_str().to_string(),
            })
            .collect()
    }
}
