use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Master switch for the entire Realms Connect feature. When false, the
// sidebar item is hidden and the gate denies with FeatureDisabled.
// Fail-closed: default false so a Flagsmith outage never exposes the
// paid feature for free.
pub struct RealmsConnectEnabled;

impl FeatureFlag for RealmsConnectEnabled {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.realms_connect.enabled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_and_default() {
        let flag = RealmsConnectEnabled;
        assert_eq!(flag.key().as_ref(), "feature.bedrock.realms_connect.enabled");
        assert!(!flag.default());
    }
}
