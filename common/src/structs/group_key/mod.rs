//! Frames on a protected group's key socket.
//!
//! The server reads only a frame's type, recipient and epoch. Every `body` is opaque to it: an
//! MLS message or an admission message that is already encrypted, public, or safe to relay.

pub mod admission_message;
pub mod frame;
pub mod notice;
pub mod payload;
pub mod recipient;

pub use admission_message::AdmissionMessage;
pub use frame::GroupKeyFrame;
pub use payload::GroupKeyPayload;
pub use recipient::GroupKeyRecipient;

/// A key socket's identity on one group, assigned by the server when the socket opens.
pub type SocketId = u64;
