use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a track is, which is what decides where it sits in the list and what it is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum TrackKind {
    Own,
    Player,
    Jukebox,
}
