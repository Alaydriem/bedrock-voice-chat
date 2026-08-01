use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Remote kill switch for Realms Connect. When false the sidebar item is
// hidden, the page shows an unavailable notice, and `bedrock_start_realms`
// refuses to start a session.
// Fail-open: Realms Connect costs nothing, so a Flagsmith outage must not
// hide it.
pub struct RealmsConnectEnabled;

impl FeatureFlag for RealmsConnectEnabled {
    type Value = bool;

    fn default(&self) -> bool {
        true
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.realms_connect.enabled")
    }
}
