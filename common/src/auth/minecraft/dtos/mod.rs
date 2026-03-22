//! Data Transfer Objects for Minecraft/Xbox Live authentication

mod access_token_response;
mod minecraft_login_response;
mod minecraft_profile_response;
mod profile;
mod xbox_auth_response;

// Module-internal exports
pub(super) use access_token_response::AccessTokenResponse;
pub(super) use minecraft_login_response::MinecraftLoginResponse;
pub(super) use minecraft_profile_response::MinecraftProfileResponse;
pub(super) use profile::ProfileResponse;
pub(super) use xbox_auth_response::XboxAuthResponse;
