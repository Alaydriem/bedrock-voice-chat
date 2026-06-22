use base64::Engine;
use common::request::CodeLoginRequest;
use common::response::LoginResponse;

use super::json_message::JsonMessage;
use super::server::TestServer;

const ENDPOINT: &str = "/api/auth/code";

pub struct NcryptfLogin;

impl NcryptfLogin {
    pub async fn perform(
        env: &TestServer,
        payload: &CodeLoginRequest,
    ) -> anyhow::Result<LoginResponse> {
        let ek: common::ncryptflib::ExportableEncryptionKeyData = env
            .noauth_client()?
            .get(format!("{}/ncryptf/ek", env.base_url))
            .send()
            .await?
            .json()
            .await?;

        let kp = common::ncryptflib::Keypair::new();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/vnd.ncryptf+json"),
        );
        headers.insert(
            "X-HashId",
            reqwest::header::HeaderValue::from_str(&ek.hash_id)?,
        );
        headers.insert(
            "X-PubKey",
            reqwest::header::HeaderValue::from_str(
                &base64::engine::general_purpose::STANDARD.encode(kp.get_public_key()),
            )?,
        );

        let resp = env
            .noauth_client()?
            .post(format!("{}{}", env.base_url, ENDPOINT))
            .headers(headers)
            .json(payload)
            .send()
            .await?;

        let status = resp.status();
        let bytes = resp.bytes().await?;
        let decoded = base64::engine::general_purpose::STANDARD.decode(&bytes)?;

        let response = common::ncryptflib::Response::from(kp.get_secret_key())?;
        let decrypted = response.decrypt(decoded, None, None)?;

        let wrapper: JsonMessage<LoginResponse> = serde_json::from_str(&decrypted)?;

        if !status.is_success() {
            anyhow::bail!(
                "ncryptf login failed: status={}, message={:?}",
                wrapper.status,
                wrapper.message
            );
        }

        wrapper
            .data
            .ok_or_else(|| anyhow::anyhow!("empty data field"))
    }
}
