//! Error types for the common crate

pub mod communication;

pub use communication::{
    CommunicationError, GameError, GenericCommunicationError, MinecraftCommunicationError,
};
