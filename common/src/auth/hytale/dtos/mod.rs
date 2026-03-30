//! Data Transfer Objects for Hytale authentication

mod device_code_response;
mod device_flow;
mod hytale_profile;
mod poll_result;
mod profile_response;
mod token_error_response;
mod token_response;

// Public exports
pub use device_flow::DeviceFlow;
pub use poll_result::PollResult;

// Crate-internal exports
pub(super) use device_code_response::DeviceCodeResponse;
pub(super) use profile_response::HytaleProfileResponse;
pub(super) use token_error_response::TokenErrorResponse;
pub(super) use token_response::TokenResponse;
