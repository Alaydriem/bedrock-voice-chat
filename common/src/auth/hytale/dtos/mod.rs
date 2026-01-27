//! Data Transfer Objects for Hytale authentication

mod device_code_response;
mod device_flow;
mod poll_result;
mod profile_response;
mod token_response;

// Public exports
pub use device_flow::DeviceFlow;
pub use poll_result::PollResult;

// Crate-internal exports
pub(super) use device_code_response::DeviceCodeResponse;
pub(super) use profile_response::HytaleProfileResponse;
pub(super) use token_response::{TokenErrorResponse, TokenResponse};
