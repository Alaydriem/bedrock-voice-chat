/// Auth seam for Minecraft OAuth — production uses MinecraftAuthProvider; tests inject fakes.
use async_trait::async_trait;
use reqwest::Url;

use super::provider::{AuthError, AuthResult};

#[async_trait]
pub trait MinecraftAuthenticator: Send + Sync {
    async fn authenticate(
        &self,
        code: String,
        redirect_uri: Url,
    ) -> Result<AuthResult, AuthError>;

    async fn authenticate_for_java_profile(
        &self,
        code: String,
        redirect_uri: Url,
    ) -> Result<String, AuthError>;
}
