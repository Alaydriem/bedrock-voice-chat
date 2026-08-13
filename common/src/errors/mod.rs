//! Error types for the common crate

pub mod chat;
pub mod communication;
pub mod discord_link_error;

pub use chat::ChatRejection;
pub use communication::{
    CommunicationError, GameError, GenericCommunicationError, HytaleCommunicationError,
    MinecraftCommunicationError,
};
pub use discord_link_error::DiscordLinkError;
