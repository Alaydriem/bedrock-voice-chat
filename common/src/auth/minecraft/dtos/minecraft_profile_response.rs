use serde::Deserialize;

/// Response from GET https://api.minecraftservices.com/minecraft/profile
#[derive(Debug, Deserialize)]
pub struct MinecraftProfileResponse {
    pub name: String,
}
