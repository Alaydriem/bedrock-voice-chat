use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::key::PlayerKey;
use crate::structs::audio::PlayerGainSettings;

/// One stored row: who, where, and what you decided about them.
///
/// The shape the Players settings pane consumes. The key stays whole here rather than being
/// flattened to a bare identity, because the pane lists rows across a server boundary the
/// mixer never sees — the mixer is handed an already-scoped projection instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerSettingsRow {
    pub key: PlayerKey,
    pub settings: PlayerGainSettings,
}

impl PlayerSettingsRow {
    pub fn new(key: PlayerKey, settings: PlayerGainSettings) -> Self {
        Self { key, settings }
    }
}
