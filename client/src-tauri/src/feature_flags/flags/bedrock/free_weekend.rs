use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Scheduled free-weekend window, toggled in Flagsmith. When true, the gate
// allows Realms Connect with no entitlement. Default false.
pub struct FreeWeekendEnabled;

impl FeatureFlag for FreeWeekendEnabled {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.free_weekend.enabled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_and_default() {
        let flag = FreeWeekendEnabled;
        assert_eq!(flag.key().as_ref(), "feature.bedrock.free_weekend.enabled");
        assert!(!flag.default());
    }
}
