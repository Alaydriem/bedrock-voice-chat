pub mod hytale;
pub mod minecraft;

pub use hytale::{poll_status as hytale_poll_status, start_device_flow as hytale_start_device_flow};
pub use minecraft::authenticate as minecraft_authenticate;

// Re-export HytaleSessionCache from structs for route mounting
pub use crate::rs::structs::HytaleSessionCache;
