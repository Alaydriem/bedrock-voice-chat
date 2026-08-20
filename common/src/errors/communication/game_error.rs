use super::{GenericCommunicationError, MinecraftCommunicationError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum GameError {
    #[error("minecraft: {0}")]
    Minecraft(MinecraftCommunicationError),

    #[error("generic: {0}")]
    Generic(GenericCommunicationError),
}
