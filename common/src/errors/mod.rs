//! Error types for the common crate

pub mod admission_error;
pub mod chat;
pub mod communication;
pub mod discord_link_error;
pub mod pairing;
pub mod peer_wire;
pub mod protect_refusal;
pub mod protected_group_error;
pub mod sframe_error;

pub use admission_error::AdmissionError;
pub use chat::ChatRejection;
pub use communication::{
    CommunicationError, GameError, GenericCommunicationError, MinecraftCommunicationError,
};
pub use discord_link_error::DiscordLinkError;
pub use pairing::PairingCodeError;
pub use peer_wire::PeerWireError;
pub use protect_refusal::ProtectRefusal;
pub use protected_group_error::ProtectedGroupError;
pub use sframe_error::SframeError;
