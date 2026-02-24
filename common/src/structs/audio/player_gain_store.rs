use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use super::player_gain_settings::PlayerGainSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainStore(pub HashMap<String, PlayerGainSettings>);

impl Default for PlayerGainStore {
    fn default() -> Self {
        Self(std::collections::HashMap::new())
    }
}
