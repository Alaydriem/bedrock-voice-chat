//! Communication errors between players

mod generic;
mod minecraft;

pub use generic::GenericCommunicationError;
pub use minecraft::MinecraftCommunicationError;

use crate::Game;

/// Game-specific error wrapper
#[derive(Debug, Clone, thiserror::Error)]
pub enum GameError {
    #[error("minecraft: {0}")]
    Minecraft(MinecraftCommunicationError),

    #[error("generic: {0}")]
    Generic(GenericCommunicationError),
}

/// Error returned when two players cannot communicate
#[derive(Debug, Clone, thiserror::Error)]
pub enum CommunicationError {
    /// Players are in different games (e.g., Minecraft vs Generic)
    #[error("game mismatch: sender={sender_game:?}, recipient={recipient_game:?}")]
    GameMismatch {
        sender_game: Game,
        recipient_game: Game,
    },

    /// Players are too far apart (common across all games)
    #[error("out of range: distance={distance:.2}, max_range={max_range:.2}")]
    OutOfRange { distance: f32, max_range: f32 },

    /// Game-specific error
    #[error("{0}")]
    Game(GameError),
}

impl CommunicationError {
    /// Convenience constructor for Minecraft errors
    pub fn minecraft(err: MinecraftCommunicationError) -> Self {
        CommunicationError::Game(GameError::Minecraft(err))
    }

    /// Convenience constructor for Generic game errors
    pub fn generic(err: GenericCommunicationError) -> Self {
        CommunicationError::Game(GameError::Generic(err))
    }
}
