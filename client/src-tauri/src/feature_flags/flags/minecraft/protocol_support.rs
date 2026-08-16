use std::borrow::Cow;

use crate::feature_flags::feature_flag::FeatureFlag;

// "Is this specific Minecraft protocol version allowed even though it's not
// in the compiled SUPPORTED_PROTOCOLS list?" Used to grant ad-hoc access to
// a brand-new Minecraft release after manual validation, before BVC
// engineering compiles in support.
//
// Flagsmith key: `feature.minecraft.protocol.<N>` where N is the raw
// `varuint` protocol number negotiated by the client (e.g. 988). Per-version
// keys give the dashboard one toggle per concept and one audit history per
// version — bumping 988 doesn't entangle with 989.
pub struct MinecraftProtocolSupport {
    pub protocol_version: i32,
}

impl FeatureFlag for MinecraftProtocolSupport {
    type Value = bool;

    fn default(&self) -> bool {
        false
    }

    fn key(&self) -> Cow<'static, str> {
        Cow::Owned(format!(
            "feature.minecraft.protocol.{}",
            self.protocol_version
        ))
    }
}
