use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// "Trust any Minecraft protocol <= this number." Optional shortcut for the
// no-op-wire-bump case where a single Flagsmith dial covers all the
// versions Mojang has shipped since the last BVC release.
//
// Flagsmith key: `feature.minecraft.max_trusted_protocol`.
//
// Recommended initial value: UNSET (no flag created in Flagsmith). The
// dashboard absence is meaningful — it means "rely on SUPPORTED_PROTOCOLS
// and per-version overrides only." Setting it today is redundant with the
// compiled allowlist already covering current versions.
pub struct MaxTrustedMinecraftProtocol;

impl FeatureFlag for MaxTrustedMinecraftProtocol {
    type Value = Option<i64>;

    fn default(&self) -> Option<i64> {
        None
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed("feature.minecraft.max_trusted_protocol")
    }
}
