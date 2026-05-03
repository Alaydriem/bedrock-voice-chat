use anyhow::{anyhow, Context};
use base64::{engine::general_purpose, Engine};
use common::ncryptflib::{ExportableEncryptionKeyData, Keypair, Response as NcryptfResponse};
use common::request::admin::{
    BanishUserRequest, ClearPermissionRequest, CreateUserRequest, GenerateCodeRequest,
    SetPermissionRequest,
};
use common::request::LoginRequest;
use common::response::admin::{
    BanishedUserResponse, CreatedUserResponse, GeneratedCodeResponse, PermissionListResponse,
};
use common::response::auth::IntrospectResponse;
use common::response::LoginResponse;
use common::structs::config::{
    HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse,
};
use common::Game;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Certificate, Client, Identity as ReqwestIdentity, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// ncryptf-wrapped response envelope used by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMessage<T> {
    pub status: u16,
    pub data: Option<T>,
    pub message: Option<String>,
}

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

    /// Unauthenticated ncryptf GET to /ncryptf/ek.
    pub async fn get_encryption_key(
        base_url: &str,
        ca_pem: Option<&str>,
    ) -> Result<ExportableEncryptionKeyData, AdminApiError> {
        let client = Self::build_unauth_client(ca_pem).await?;
        let url = format!("{}/ncryptf/ek", base_url.trim_end_matches('/'));
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_json(resp).await
    }

    /// Unauthenticated ncryptf POST to /api/auth/minecraft.
    pub async fn login_minecraft(
        base_url: &str,
        ca_pem: Option<&str>,
        code: &str,
        redirect_uri: &str,
    ) -> Result<LoginResponse, AdminApiError> {
        let client = Self::build_unauth_client(ca_pem).await?;
        let ek = Self::get_encryption_key(base_url, ca_pem).await?;
        let kp = Keypair::new();

        let payload = LoginRequest {
            code: code.into(),
            redirect_uri: redirect_uri.into(),
        };

        let url = format!("{}/api/auth/minecraft", base_url.trim_end_matches('/'));
        let resp = Self::ncryptf_post(&client, &url, &ek, &kp, &payload).await?;
        Self::parse_ncryptf_json(resp, &kp).await
    }

    /// Unauthenticated ncryptf POST to /api/auth/hytale/start-device-flow.
    pub async fn start_hytale_device_flow(
        base_url: &str,
        ca_pem: Option<&str>,
    ) -> Result<HytaleDeviceFlowStartResponse, AdminApiError> {
        let client = Self::build_unauth_client(ca_pem).await?;
        let ek = Self::get_encryption_key(base_url, ca_pem).await?;
        let kp = Keypair::new();

        let url = format!(
            "{}/api/auth/hytale/start-device-flow",
            base_url.trim_end_matches('/')
        );
        let resp = Self::ncryptf_post(&client, &url, &ek, &kp, &serde_json::json!({})).await?;
        Self::parse_ncryptf_json(resp, &kp).await
    }

    /// Unauthenticated ncryptf GET to /api/auth/hytale/status.
    pub async fn poll_hytale_status(
        base_url: &str,
        ca_pem: Option<&str>,
        session_id: &str,
    ) -> Result<HytaleDeviceFlowStatusResponse, AdminApiError> {
        let client = Self::build_unauth_client(ca_pem).await?;
        let ek = Self::get_encryption_key(base_url, ca_pem).await?;
        let kp = Keypair::new();

        let url = format!(
            "{}/api/auth/hytale/status?session_id={}",
            base_url.trim_end_matches('/'),
            session_id
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/vnd.ncryptf+json"),
        );
        headers.insert(
            "X-HashId",
            HeaderValue::from_str(&ek.hash_id)
                .map_err(|e| AdminApiError::Transport(anyhow!(e)))?,
        );
        headers.insert(
            "X-PubKey",
            HeaderValue::from_str(&general_purpose::STANDARD.encode(kp.get_public_key()))
                .map_err(|e| AdminApiError::Transport(anyhow!(e)))?,
        );

        let resp = client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))?;
        Self::parse_ncryptf_json(resp, &kp).await
    }

    async fn build_unauth_client(ca_pem: Option<&str>) -> Result<Client, AdminApiError> {
        let mut builder = Client::builder().use_rustls_tls().https_only(true);
        if let Some(ca) = ca_pem {
            let ca_cert = Certificate::from_pem(ca.as_bytes())
                .context("parse CA cert")
                .map_err(AdminApiError::Transport)?;
            builder = builder.add_root_certificate(ca_cert);
        }
        builder
            .build()
            .context("build login reqwest client")
            .map_err(AdminApiError::Transport)
    }

    async fn ncryptf_post(
        client: &Client,
        url: &str,
        ek: &ExportableEncryptionKeyData,
        kp: &Keypair,
        body: &impl Serialize,
    ) -> Result<reqwest::Response, AdminApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/vnd.ncryptf+json"),
        );
        headers.insert(
            "X-HashId",
            HeaderValue::from_str(&ek.hash_id)
                .map_err(|e| AdminApiError::Transport(anyhow!(e)))?,
        );
        headers.insert(
            "X-PubKey",
            HeaderValue::from_str(&general_purpose::STANDARD.encode(kp.get_public_key()))
                .map_err(|e| AdminApiError::Transport(anyhow!(e)))?,
        );

        client
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!(e)))
    }

    async fn parse_ncryptf_json<T: DeserializeOwned>(
        resp: reqwest::Response,
        kp: &Keypair,
    ) -> Result<T, AdminApiError> {
        let status = resp.status();
        if !status.is_success() {
            return Err(Self::map_error(status, resp).await);
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AdminApiError::Transport(anyhow!("read response bytes: {}", e)))?;
        let decoded = general_purpose::STANDARD
            .decode(&bytes)
            .map_err(|e| AdminApiError::Transport(anyhow!("base64 decode: {}", e)))?;

        let response = NcryptfResponse::from(kp.get_secret_key())
            .map_err(|e| AdminApiError::Transport(anyhow!("ncryptf response: {}", e)))?;
        let decrypted = response
            .decrypt(decoded, None, None)
            .map_err(|e| AdminApiError::Transport(anyhow!("ncryptf decrypt: {}", e)))?;

        let wrapper: JsonMessage<T> = serde_json::from_str(&decrypted)
            .map_err(|e| AdminApiError::Transport(anyhow!("json parse: {}", e)))?;
        wrapper
            .data
            .ok_or_else(|| AdminApiError::Transport(anyhow!("empty data field in response")))
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
