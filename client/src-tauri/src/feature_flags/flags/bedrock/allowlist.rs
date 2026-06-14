use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// Per-install access grant, targeted by install_id in Flagsmith (the flag
// evaluation targeting key is the install_id). Lets specific installs — test
// accounts, comped users — use Realms Connect without a store purchase.
// Default false.
pub struct RealmsAllowlisted;

impl FeatureFlag for RealmsAllowlisted {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.bedrock.realms_connect.allowlist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_and_default() {
        let flag = RealmsAllowlisted;
        assert_eq!(
            flag.key().as_ref(),
            "feature.bedrock.realms_connect.allowlist"
        );
        assert!(!flag.default());
    }
}
