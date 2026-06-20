use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Whether the Bedrock connect section (Proxy + Realms) is visible in the sidebar.
pub struct BedrockConnectEnabled;

impl FeatureFlag for BedrockConnectEnabled {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.connect-enabled")
    }
}
