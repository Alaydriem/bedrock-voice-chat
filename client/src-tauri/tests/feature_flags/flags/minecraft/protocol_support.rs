use bvc_client_lib::feature_flags::FeatureFlag;
use bvc_client_lib::feature_flags::flags::minecraft::MinecraftProtocolSupport;

#[test]
fn the_key_carries_the_raw_protocol_number() {
    let flag = MinecraftProtocolSupport {
        protocol_version: 988,
    };
    assert_eq!(flag.key().as_ref(), "feature.minecraft.protocol.988");
}

#[test]
fn an_unlisted_protocol_is_rejected_by_default() {
    let flag = MinecraftProtocolSupport {
        protocol_version: 999,
    };
    assert!(!flag.default());
}
