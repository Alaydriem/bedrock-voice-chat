use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigAge {
    #[serde(default)]
    pub minimum: Option<u8>,
}

impl ApiConfigAge {
    // Inclusive bounds an enforced minimum age is held within: never below the
    // COPPA-style floor, never above the adult ceiling.
    pub const FLOOR: u8 = 13;
    pub const CEILING: u8 = 18;
    // Operator sentinel that turns enforcement off.
    pub const DISABLED: u8 = 0;

    // Resolve an operator-supplied minimum into the wire value. `DISABLED` (0)
    // disables enforcement (`None`); any other value enforces, clamped into
    // [FLOOR, CEILING]. An omitted server config defaults to FLOOR upstream, so
    // the default behavior is enforcement at 13.
    pub fn from_minimum(minimum: u8) -> Self {
        let minimum = if minimum == Self::DISABLED {
            None
        } else {
            Some(minimum.clamp(Self::FLOOR, Self::CEILING))
        };
        Self { minimum }
    }
}
