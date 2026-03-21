use serde::Deserialize;

use super::feature::FlagsmithFeature;
use super::value::FlagsmithFlagValue;

#[derive(Debug, Clone)]
pub(crate) struct FlagsmithFlag {
    pub enabled: bool,
    pub feature: FlagsmithFeature,
    pub value: Option<FlagsmithFlagValue>,
}

#[derive(Deserialize)]
struct RawFlagsmithFlag {
    enabled: bool,
    feature: FlagsmithFeature,
    feature_state_value: Option<serde_json::Value>,
}

impl<'de> Deserialize<'de> for FlagsmithFlag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawFlagsmithFlag::deserialize(deserializer)?;
        Ok(Self {
            enabled: raw.enabled,
            feature: raw.feature,
            value: raw.feature_state_value.as_ref().map(FlagsmithFlagValue::from_json),
        })
    }
}
