use common::bedrock_protocol::{AdvertisedVersion, ProtocolVersion};

use crate::feature_flags::FeatureFlagService;
use crate::feature_flags::flags::bedrock::RealmsAdvertisedProtocol;

// Decides which `AdvertisedVersion` the proxy hands to the bedrock-protocol
// listener for a given backend. The two backends differ because only one has
// a probe address: a direct backend can be sniffed, a Realm cannot.
pub struct AdvertisedVersionResolver;

impl AdvertisedVersionResolver {
    // Direct backends: an explicit UI selection pins the advertised version;
    // its absence means `Auto`, which mirrors the real backend once the proxy
    // is given its `backend_probe_addr`.
    pub fn proxy(selected: Option<u32>) -> AdvertisedVersion {
        match selected {
            Some(protocol) => AdvertisedVersion::Fixed(ProtocolVersion(protocol)),
            None => AdvertisedVersion::Auto,
        }
    }

    // Realm backends have no probe address, so `Auto` would silently fall back
    // to `RELEASED_LATEST` with no operator control. Pin it from the remote
    // flag instead (whose own default is the compiled `RELEASED_LATEST`), so a
    // new Realms release can be tracked without shipping a client.
    pub async fn realms(flags: &FeatureFlagService) -> AdvertisedVersion {
        let protocol = flags
            .get(RealmsAdvertisedProtocol)
            .await
            .map(|raw| ProtocolVersion(raw as u32))
            .unwrap_or(ProtocolVersion::RELEASED_LATEST);
        AdvertisedVersion::Fixed(protocol)
    }
}
