//! Data Transfer Objects for Minecraft/Xbox Live authentication

mod access_token_response;
mod profile_response;
mod xbox_auth_response;

// Module-internal exports
pub(super) use access_token_response::AccessTokenResponse;
pub(super) use profile_response::ProfileResponse;
pub(super) use xbox_auth_response::XboxAuthResponse;
