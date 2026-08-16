//! The supported way to speak the BVC peer wire from another process.
//!
//! Everything here is a shell over `bvc-relay`: the session, the reconnect and
//! the wire live there, so a second language binding is a second set of
//! generated files rather than a second implementation.

pub mod config;
pub mod error;
pub mod frame;
pub mod peer;

pub use config::SdkConfig;
pub use error::SdkError;
pub use frame::SdkFrame;
pub use peer::BvcPeer;

uniffi::setup_scaffolding!();
