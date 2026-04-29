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
use reqwest::{Certificate, Client, Identity as ReqwestIdentity, Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::commands::admin_api_error::AdminApiError;
use crate::identity::{Identity, IdentityResolver, IdentityStore};

pub struct AdminApiClient {
    base_url: String,
    http: Client,
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

    pub fn from_active_identity(explicit: Option<&str>) -> Result<Self, anyhow::Error> {
        let slot = IdentityResolver::active(explicit)?;
        let identity = IdentityStore::load(&slot)?;
        Self::new(&identity)
    }

    fn build(
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

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Single dispatch point for every admin call. Body is optional (None for GET/DELETE-without-body).
    async fn request<Req, Res>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<Res, AdminApiError>
    where
        Req: Serialize + ?Sized,
        Res: DeserializeOwned,
    {
        let mut req = self.http.request(method, self.url(path));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_json(resp).await
    }

    /// Like `request`, but the route returns no body (204 / status-only).
    async fn request_unit<Req>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<(), AdminApiError>
    where
        Req: Serialize + ?Sized,
    {
        let mut req = self.http.request(method, self.url(path));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        Err(Self::map_error(status, resp).await)
    }

    pub async fn create_user(
        &self,
        req: &CreateUserRequest,
    ) -> Result<CreatedUserResponse, AdminApiError> {
        self.request(Method::POST, "/api/admin/user", Some(req)).await
    }

    pub async fn banish_user(
        &self,
        req: &BanishUserRequest,
    ) -> Result<BanishedUserResponse, AdminApiError> {
        self.request(Method::PATCH, "/api/admin/user/banish", Some(req))
            .await
    }

    pub async fn generate_code(
        &self,
        req: &GenerateCodeRequest,
    ) -> Result<GeneratedCodeResponse, AdminApiError> {
        self.request(Method::POST, "/api/admin/user/code", Some(req))
            .await
    }

    pub async fn set_permission(&self, req: &SetPermissionRequest) -> Result<(), AdminApiError> {
        self.request_unit(Method::PUT, "/api/admin/permission", Some(req))
            .await
    }

    pub async fn clear_permission(
        &self,
        req: &ClearPermissionRequest,
    ) -> Result<(), AdminApiError> {
        self.request_unit(Method::DELETE, "/api/admin/permission", Some(req))
            .await
    }

    pub async fn list_permissions(
        &self,
        gamertag: &str,
        game: &Game,
    ) -> Result<PermissionListResponse, AdminApiError> {
        let path = format!(
            "/api/admin/permission/{}/{}",
            game.as_str(),
            Self::encode_segment(gamertag)
        );
        self.request::<(), _>(Method::GET, &path, None).await
    }

    pub async fn introspect(&self) -> Result<IntrospectResponse, AdminApiError> {
        self.request::<(), _>(Method::GET, "/api/auth/introspect", None)
            .await
    }

    /// Used by `bvc login` before any client cert exists. Server cert verification still required.
    pub async fn login_with_code(
        base_url: &str,
        ca_pem: Option<&str>,
        req: &CodeLoginRequest,
    ) -> Result<LoginResponse, AdminApiError> {
        let mut builder = Client::builder().use_rustls_tls().https_only(true);
        if let Some(ca) = ca_pem {
            let ca_cert = Certificate::from_pem(ca.as_bytes())
                .context("parse CA cert")
                .map_err(AdminApiError::Transport)?;
            builder = builder.add_root_certificate(ca_cert);
        }
        let client = builder
            .build()
            .context("build login reqwest client")
            .map_err(AdminApiError::Transport)?;

        let url = format!("{}/api/auth/code/json", base_url.trim_end_matches('/'));
        let resp = client
            .post(url)
            .json(req)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_json(resp).await
    }

    async fn parse_json<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, AdminApiError> {
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<T>()
                .await
                .map_err(|e| AdminApiError::Transport(anyhow!("decode response body: {}", e)));
        }
        Err(Self::map_error(status, resp).await)
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
}
