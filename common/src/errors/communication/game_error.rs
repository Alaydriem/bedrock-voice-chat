use super::{GenericCommunicationError, HytaleCommunicationError, MinecraftCommunicationError};

/// Game-specific error wrapper
#[derive(Debug, Clone, thiserror::Error)]
pub enum GameError {
    #[error("minecraft: {0}")]
    Minecraft(MinecraftCommunicationError),

    #[error("hytale: {0}")]
    Hytale(HytaleCommunicationError),

    #[error("generic: {0}")]
    Generic(GenericCommunicationError),
}
