//! Minecraft-specific communication errors

use crate::game_data::Dimension;

/// Minecraft-specific communication errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MinecraftCommunicationError {
    #[error("dimension mismatch: sender={sender:?}, recipient={recipient:?}")]
    DimensionMismatch {
        sender: Dimension,
        recipient: Dimension,
    },
}
