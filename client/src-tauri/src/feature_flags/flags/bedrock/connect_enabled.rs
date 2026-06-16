use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Whether the Minecraft Bedrock connect feature (Proxy + Realms settings) is
// exposed in this build. Evaluated against the Flagsmith environment selected
// by release channel at build time, so testing channels can hide it while
// production controls it from the dashboard.
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
