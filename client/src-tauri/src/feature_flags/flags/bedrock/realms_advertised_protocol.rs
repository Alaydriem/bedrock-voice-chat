use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// The Bedrock protocol version the proxy advertises to clients on a Realms
// Connect session, controlled remotely. Realms always run the latest public
// release, and the proxy is a transparent relay, so its advertised version
// must track whatever Mojang has pushed to Realms — a lever the operator
// needs to turn the moment a new release lands, without shipping a client.
//
// Flagsmith key: `feature.bedrock.realms_connect.advertised_protocol`, an
// integer set to the raw `varuint` protocol number (e.g. 1001 for 1.26.30).
//
// The default is the compiled static fallback used when the flag is unset or
// Flagsmith is unreachable: `RELEASED_LATEST` at build time.
pub struct RealmsAdvertisedProtocol;

impl FeatureFlag for RealmsAdvertisedProtocol {
    type Value = Option<i64>;

    fn default(&self) -> Option<i64> {
        Some(common::bedrock_protocol::ProtocolVersion::RELEASED_LATEST.0 as i64)
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.realms_connect.advertised_protocol")
    }
}
