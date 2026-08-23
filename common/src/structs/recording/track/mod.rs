use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod kind;

pub use kind::TrackKind;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RecordingTrack {
    // Every WAL key that lands on this one output file. One for a voice; the jukebox
    // brings as many as it played.
    pub keys: Vec<String>,
    // What a person sees, and what the file is named after.
    pub display: String,
    pub kind: TrackKind,
}
