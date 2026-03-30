use crate::Game;
use super::{GameError, GenericCommunicationError, HytaleCommunicationError, MinecraftCommunicationError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum CommunicationError {
    #[error("game mismatch: sender={sender_game:?}, recipient={recipient_game:?}")]
    GameMismatch {
        sender_game: Game,
        recipient_game: Game,
    },

    #[error("out of range: distance={distance:.2}, max_range={max_range:.2}")]
    OutOfRange { distance: f32, max_range: f32 },

    #[error("{0}")]
    Game(GameError),
}

impl CommunicationError {
    pub fn minecraft(err: MinecraftCommunicationError) -> Self {
        CommunicationError::Game(GameError::Minecraft(err))
    }

    pub fn hytale(err: HytaleCommunicationError) -> Self {
        CommunicationError::Game(GameError::Hytale(err))
    }

    pub fn generic(err: GenericCommunicationError) -> Self {
        CommunicationError::Game(GameError::Generic(err))
    }
}
