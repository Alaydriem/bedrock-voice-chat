use std::borrow::Cow;

use bvc_client_lib::feature_flags::FeatureFlag;
use bvc_client_lib::feature_flags::flags::minecraft::MaxTrustedMinecraftProtocol;

#[test]
fn the_key_is_a_static_borrow() {
    let key = MaxTrustedMinecraftProtocol.key();
    assert_eq!(key.as_ref(), "feature.minecraft.max_trusted_protocol");
    assert!(matches!(key, Cow::Borrowed(_)));
}

#[test]
fn the_flag_is_unset_by_default() {
    assert!(MaxTrustedMinecraftProtocol.default().is_none());
}
