use anyhow::{anyhow, Context};
use common::request::admin::{
    BanishUserRequest, ClearPermissionRequest, CreateUserRequest, GenerateCodeRequest,
    SetPermissionRequest,
};
use common::request::CodeLoginRequest;
use common::response::admin::{
    BanishedUserResponse, CreatedUserResponse, GeneratedCodeResponse, PermissionListResponse,
};
use common::response::auth::IntrospectResponse;
use common::response::LoginResponse;
use common::Game;
use reqwest::{Certificate, Client, Identity as ReqwestIdentity, StatusCode};
use serde::de::DeserializeOwned;

use crate::commands::identity::{Identity, IdentityResolver, IdentityStore};

pub struct AdminApiClient {
    base_url: String,
    http: Client,
}

#[derive(Debug)]
pub enum AdminApiError {
    NotFound,
    Conflict,
    Forbidden,
    BadRequest(String),
    Unexpected(StatusCode, String),
    Transport(anyhow::Error),
}

impl std::fmt::Display for AdminApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminApiError::NotFound => write!(f, "not found"),
            AdminApiError::Conflict => write!(f, "conflict"),
            AdminApiError::Forbidden => write!(f, "forbidden"),
            AdminApiError::BadRequest(b) => write!(f, "bad request: {}", b),
            AdminApiError::Unexpected(s, b) => write!(f, "unexpected status {}: {}", s, b),
            AdminApiError::Transport(e) => write!(f, "transport error: {}", e),
        }
    }
}

impl std::error::Error for AdminApiError {}

impl From<anyhow::Error> for AdminApiError {
    fn from(e: anyhow::Error) -> Self {
        AdminApiError::Transport(e)
    }
}

impl AdminApiClient {
    pub fn new(identity: &Identity) -> Result<Self, anyhow::Error> {
        Self::build(
            &identity.server_url,
            &identity.cert_pem,
            &identity.key_pem,
            &identity.ca_pem,
        )
    }

    /// Resolve the active identity (per --identity / BVC_IDENTITY / single-stored), load it
    /// from the keychain/file backend, and build a client. Used by every admin subcommand.
    pub fn from_active_identity(explicit: Option<&str>) -> Result<Self, anyhow::Error> {
        let slot = IdentityResolver::active(explicit)?;
        let identity = IdentityStore::load(&slot)?;
        Self::new(&identity)
    }

    /// Lower-level constructor used by `bvc login`, where we don't yet have a stored Identity.
    pub fn build(
        base_url: &str,
        cert_pem: &str,
        key_pem: &str,
        ca_pem: &str,
    ) -> Result<Self, anyhow::Error> {
        let mut combined = Vec::with_capacity(cert_pem.len() + key_pem.len() + 1);
        combined.extend_from_slice(cert_pem.as_bytes());
        combined.push(b'\n');
        combined.extend_from_slice(key_pem.as_bytes());

        let client_identity = ReqwestIdentity::from_pem(&combined)
            .context("parse client cert/key as reqwest Identity")?;
        let ca_cert = Certificate::from_pem(ca_pem.as_bytes()).context("parse CA cert")?;

        let http = Client::builder()
            .use_rustls_tls()
            .identity(client_identity)
            .add_root_certificate(ca_cert)
            .https_only(true)
            .build()
            .context("build reqwest client")?;

        Ok(AdminApiClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    pub fn build_for_login(
        base_url: &str,
        ca_pem: Option<&str>,
    ) -> Result<Client, anyhow::Error> {
        let mut builder = Client::builder().use_rustls_tls().https_only(true);
        if let Some(ca) = ca_pem {
            let ca_cert = Certificate::from_pem(ca.as_bytes()).context("parse CA cert")?;
            builder = builder.add_root_certificate(ca_cert);
        }
        let _ = base_url;
        builder.build().context("build login reqwest client")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn create_user(
        &self,
        req: &CreateUserRequest,
    ) -> Result<CreatedUserResponse, AdminApiError> {
        let resp = self
            .http
            .post(self.url("/api/admin/user"))
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    pub async fn banish_user(
        &self,
        req: &BanishUserRequest,
    ) -> Result<BanishedUserResponse, AdminApiError> {
        let resp = self
            .http
            .patch(self.url("/api/admin/user/banish"))
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    pub async fn generate_code(
        &self,
        req: &GenerateCodeRequest,
    ) -> Result<GeneratedCodeResponse, AdminApiError> {
        let resp = self
            .http
            .post(self.url("/api/admin/user/code"))
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    pub async fn set_permission(&self, req: &SetPermissionRequest) -> Result<(), AdminApiError> {
        let resp = self
            .http
            .put(self.url("/api/admin/permission"))
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_unit(resp).await
    }

    pub async fn clear_permission(
        &self,
        req: &ClearPermissionRequest,
    ) -> Result<(), AdminApiError> {
        let resp = self
            .http
            .delete(self.url("/api/admin/permission"))
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_unit(resp).await
    }

    pub async fn list_permissions(
        &self,
        gamertag: &str,
        game: &Game,
    ) -> Result<PermissionListResponse, AdminApiError> {
        let url = self.url(&format!(
            "/api/admin/permission/{}/{}",
            game.as_str(),
            Self::encode_segment(gamertag)
        ));
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    pub async fn introspect(&self) -> Result<IntrospectResponse, AdminApiError> {
        let resp = self
            .http
            .get(self.url("/api/auth/introspect"))
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    /// Used by `bvc login`. Hits the plain-JSON code-login endpoint, no client cert needed.
    pub async fn login_with_code(
        base_url: &str,
        ca_pem: Option<&str>,
        req: &CodeLoginRequest,
    ) -> Result<LoginResponse, AdminApiError> {
        let client = Self::build_for_login(base_url, ca_pem).map_err(AdminApiError::Transport)?;
        let url = format!(
            "{}/api/auth/code/json",
            base_url.trim_end_matches('/')
        );
        let resp = client
            .post(url)
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse(resp).await
    }

    async fn parse<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, AdminApiError> {
        let status = resp.status();
        if status.is_success() {
            let body = resp
                .json::<T>()
                .await
                .map_err(|e| AdminApiError::Transport(anyhow!("decode response body: {}", e)))?;
            return Ok(body);
        }
        Err(Self::map_error(status, resp).await)
    }

    async fn parse_unit(resp: reqwest::Response) -> Result<(), AdminApiError> {
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        Err(Self::map_error(status, resp).await)
    }

    fn encode_segment(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char);
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }

    async fn map_error(status: StatusCode, resp: reqwest::Response) -> AdminApiError {
        let body = resp.text().await.unwrap_or_default();
        match status {
            StatusCode::NOT_FOUND => AdminApiError::NotFound,
            StatusCode::CONFLICT => AdminApiError::Conflict,
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => AdminApiError::Forbidden,
            StatusCode::BAD_REQUEST => AdminApiError::BadRequest(body),
            other => AdminApiError::Unexpected(other, body),
        }
    }
}
