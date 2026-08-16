use serde::{Deserialize, Serialize};

// What a peer is permitted to do on a link.
//
// Declared per peer in `config.hcl` and echoed back on the control stream, so a
// bridge can compare what it was granted against what it expects and fail loudly
// rather than meeting a missing capability as silence on the audio path.
//
// `QueryAudio` and `ServeAudio` are reserved and unimplemented. They described a
// peer fetching an audio file it did not hold, which nothing does any more —
// jukebox audio is decoded where it is played and reaches a peer as ordinary
// frames. Granting either changes no behaviour.
//
// Postcard encodes a variant as its index, so this list is append-only — which is
// also why the two stay rather than being removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    CarrySpeakers,
    QueryAudio,
    ServeAudio,
}

impl Capability {
    // The spelling used in `config.hcl`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CarrySpeakers => "carry_speakers",
            Self::QueryAudio => "query_audio",
            Self::ServeAudio => "serve_audio",
        }
    }

    // Inverse of `as_str`. `None` for anything unrecognised, so a typo in an
    // operator's config fails at startup instead of silently granting nothing.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "carry_speakers" => Some(Self::CarrySpeakers),
            "query_audio" => Some(Self::QueryAudio),
            "serve_audio" => Some(Self::ServeAudio),
            _ => None,
        }
    }
}
