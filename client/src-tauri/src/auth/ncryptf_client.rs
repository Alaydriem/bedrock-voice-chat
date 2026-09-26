use common::ncryptflib::ExportableEncryptionKeyData;

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

        let mut builder = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(config.timeout())
            .connect_timeout(config.connect_timeout())
            .danger_accept_invalid_certs(false);

        if let Some(local) = config.local_address() {
            builder = builder.local_address(local);
        }

        // Matches `api::Client`. A development server presents a certificate no root signed,
        // and a debug build that can reach the API but cannot sign in is a difference that
        // serves nobody. `debug_assertions` is the cfg cargo actually sets; the `dev` this
        // replaced is not one, so the branch never compiled.
        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder.build().unwrap()
    }
}
