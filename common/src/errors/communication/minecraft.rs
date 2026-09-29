//! Minecraft-specific communication errors

use crate::game_data::Dimension;

/// Minecraft-specific communication errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MinecraftCommunicationError {
    // Carries no world ids: this is raised per recipient per audio frame, and copying two
    // UUID strings into every rejection was an allocation on the routing hot path.
    #[error("world mismatch")]
    WorldMismatch,
    #[error("dimension mismatch: sender={sender:?}, recipient={recipient:?}")]
    DimensionMismatch {
        sender: Dimension,
        recipient: Dimension,
    },
    #[error("spectator cannot be heard by non-spectator")]
    SpectatorInaudible,
}
