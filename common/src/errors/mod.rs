//! Error types for the common crate

pub mod chat;
pub mod communication;
pub mod discord_link_error;
pub mod peer_wire;

pub use chat::ChatRejection;
pub use communication::{
    CommunicationError, GameError, GenericCommunicationError, MinecraftCommunicationError,
};
pub use discord_link_error::DiscordLinkError;
pub use peer_wire::PeerWireError;
