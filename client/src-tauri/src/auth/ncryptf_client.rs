use common::ncryptflib::ExportableEncryptionKeyData;
use common::rustls::{BvcClientTls, VerifierMode};

use common::structs::reachability::AddressFamilyPreference;
use tauri_plugin_http::reqwest;

use crate::auth::LoginClientConfig;

pub const CONFIG_ENDPOINT: &'static str = "api/config";
pub const AUTH_ENDPOINT: &'static str = "api/auth/minecraft";
pub const NCRYPTF_EK_ENDPOINT: &'static str = "ncryptf/ek";

pub struct NcryptfClient;

impl NcryptfClient {
    pub(crate) async fn get_ek(
        server: String,
        preference: AddressFamilyPreference,
    ) -> Result<ExportableEncryptionKeyData, anyhow::Error> {
        let endpoint = super::ServerEndpoint::join(&server, NCRYPTF_EK_ENDPOINT);

        let client = Self::get_reqwest_client(preference);
        let ek: ExportableEncryptionKeyData = client
            .get(endpoint)
            .send()
            .await?
            .json::<ExportableEncryptionKeyData>()
            .await?;

        Ok(ek)
    }

    pub(crate) fn get_reqwest_client(preference: AddressFamilyPreference) -> reqwest::Client {
        let config = LoginClientConfig::new(preference);

        let mut builder = BvcClientTls::http_client_builder(None, None, VerifierMode::for_build())
            .expect("the built-in TLS configuration must build")
            .timeout(config.timeout())
            .connect_timeout(config.connect_timeout());

        if let Some(local) = config.local_address() {
            builder = builder.local_address(local);
        }

        builder.build().unwrap()
    }
}
