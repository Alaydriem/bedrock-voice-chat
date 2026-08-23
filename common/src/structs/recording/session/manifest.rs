use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SessionManifest {
    pub session_id: String,
    pub start_timestamp: u64,
    pub end_timestamp: Option<u64>,
    pub duration_ms: Option<u64>,
    pub emitter_player: String,
    pub participants: Vec<String>,
    #[serde(default)]
    pub jukebox_participants: Vec<String>,
    pub created_at: String,
    #[serde(default)]
    pub recording_version: Option<String>,
    // What the recording is called, once somebody names it. Absent on every session
    // recorded before renaming existed, so it defaults rather than failing to
    // deserialise a manifest already on disk.
    //
    // No fallback is computed here. An unnamed session is shown by when it happened, and
    // `created_at` is a unix-seconds string — turning that into something a person can
    // read needs their locale and their timezone, neither of which this side has.
    #[serde(default)]
    pub name: Option<String>,
}
