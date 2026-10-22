use serde::{Deserialize, Serialize};

use super::jukebox_metadata::JukeboxMetadata;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AudioFrameMetadata {
    Jukebox(JukeboxMetadata),
    /// `data` is an SFrame ciphertext for a protected group, not Opus.
    Sealed,
}
