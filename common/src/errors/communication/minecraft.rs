//! Minecraft-specific communication errors

use crate::game_data::Dimension;

/// Minecraft-specific communication errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MinecraftCommunicationError {
    #[error("world mismatch: sender={sender_world}, recipient={recipient_world}")]
    WorldMismatch {
        sender_world: String,
        recipient_world: String,
    },
    #[error("dimension mismatch: sender={sender:?}, recipient={recipient:?}")]
    DimensionMismatch {
        sender: Dimension,
        recipient: Dimension,
    },
    #[error("spectator cannot be heard by non-spectator")]
    SpectatorInaudible,
}
