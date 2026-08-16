use serde::{Deserialize, Serialize};

use crate::PlayerEnum;

// One encoded audio frame from a speaker the sending peer owns.
//
// The speaker travels on the frame rather than being resolved locally. A
// receiving server has no position feed covering another server's players, and a
// bridge's speakers are not its clients at all, so there is nothing to look up.
//
// `timestamp_ms` is supplied by the caller rather than read from the clock here,
// which is what makes a frame a value rather than a moment and lets its encoding
// be pinned.
//
// No `PartialEq`: `PlayerEnum` does not implement it, and equality is not what
// this type is checked by — its encoding is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceFrame {
    pub speaker: PlayerEnum,
    pub sample_rate: u32,

    #[serde(with = "serde_bytes")]
    pub opus: Vec<u8>,

    pub timestamp_ms: i64,
    pub spatial: bool,

    // The playback this frame belongs to, when it is jukebox audio rather than
    // speech.
    //
    // Carried so a receiver does not have to recognise jukebox audio by the
    // speaker's name. That prefix is a real convention on this side of the wire,
    // but across a published API it is an undocumented one, and a player named
    // `jukebox-` would inherit music gain from it.
    //
    // The event id rather than a flag, because concurrent playbacks are kept on
    // separate sinks and the id is what separates them. Position and dimension
    // are not repeated here — they already travel on the speaker.
    pub jukebox: Option<String>,
}
