use serde::Deserialize;

/// Response from POST https://api.minecraftservices.com/authentication/login_with_xbox
#[derive(Debug, Deserialize)]
pub struct MinecraftLoginResponse {
    pub access_token: String,
}
