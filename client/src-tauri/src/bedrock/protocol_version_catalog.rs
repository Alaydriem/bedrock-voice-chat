use common::bedrock_protocol::ProtocolVersion;
use common::structs::bedrock::ProtocolVersionOption;

// The advertised-version choices offered in the UI. Every generated protocol
// that maps to a real public Minecraft version string is listed; protocols
// without one (dev/preview builds) are omitted so the dropdown only offers
// releases a player could actually be running.
pub struct ProtocolVersionCatalog;

impl ProtocolVersionCatalog {
    pub fn released() -> Vec<ProtocolVersionOption> {
        ProtocolVersion::GENERATED_ALL
            .iter()
            .filter_map(|version| {
                let label = version.client_version_str();
                if label == "unknown" {
                    return None;
                }
                Some(ProtocolVersionOption {
                    protocol: version.0,
                    label: label.to_string(),
                })
            })
            .collect()
    }
}
