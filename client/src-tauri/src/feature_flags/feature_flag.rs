use std::borrow::Cow;

use crate::feature_flags::flagsmith_value::FlagsmithValue;

// One feature flag definition = one struct that implements `FeatureFlag`.
// The struct holds any parameters needed to build the Flagsmith key
// (e.g. a protocol version integer); the trait carries the value type, the
// default returned on miss / Flagsmith unreachable, and the key itself.
//
// To find every flag the codebase reads, grep `impl FeatureFlag for`.
//
// Each impl lives in its own file under `feature_flags/flags/<domain>/`.
pub trait FeatureFlag {
    // Value type this flag returns. Boolean toggles use `bool`; integer
    // dials use `Option<i64>` (`None` = flag unset in Flagsmith).
    type Value: FlagsmithValue;

    // Value returned when Flagsmith is unreachable or the flag is unset.
    // Pick the conservative answer — usually `false` / `None`.
    fn default(&self) -> Self::Value;

    // Flagsmith key. Static flags return `Cow::Borrowed` (zero alloc);
    // dynamic flags whose key depends on struct fields return `Cow::Owned`
    // (one short `format!` per check, fine at handshake rate).
    fn key(&self) -> Cow<'static, str>;
}
