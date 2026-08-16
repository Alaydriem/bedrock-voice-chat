//! The JSON envelope every HTTP response is wrapped in

mod error;
mod message;

pub use error::JsonError;
pub use message::JsonMessage;
