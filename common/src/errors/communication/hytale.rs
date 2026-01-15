//! Hytale-specific communication errors

use crate::game_data::hytale::Dimension;

/// Hytale-specific errors when players cannot communicate
#[derive(Debug, Clone, thiserror::Error)]
pub enum HytaleCommunicationError {
    /// Players are in different worlds
    #[error("world mismatch: sender={sender_world}, recipient={recipient_world}")]
    WorldMismatch {
        sender_world: String,
        recipient_world: String,
    },

    /// Players are in different dimensions within the same world
    #[error("dimension mismatch: sender={sender:?}, recipient={recipient:?}")]
    DimensionMismatch {
        sender: Dimension,
        recipient: Dimension,
    },
}
