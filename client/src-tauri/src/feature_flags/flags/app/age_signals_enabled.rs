use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Master switch for the platform-native age-signals gate. When false the
// gate is skipped and the user proceeds. Fail-closed default: a Flagsmith
// outage disables the gate rather than blocking everyone.
pub struct AgeSignalsEnabled;

impl FeatureFlag for AgeSignalsEnabled {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.age-signals.enabled")
    }
}
