use super::super::feature::FlagsmithFeature;

use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct RawFlagsmithFlag {
    pub(super) enabled: bool,
    pub(super) feature: FlagsmithFeature,
    pub(super) feature_state_value: Option<serde_json::Value>,
}
