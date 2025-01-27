use common::ncryptflib::rocket::ExportableEncryptionKeyData;

use tauri_plugin_http::reqwest;
use std::time::Duration;

pub const CONFIG_ENDPOINT: &'static str = "/api/config";
pub const AUTH_ENDPOINT: &'static str = "/api/auth";
pub const NCRYPTF_EK_ENDPOINT: &'static str = "/ncryptf/ek";

pub(crate) async fn get_ek(
    server: String
) -> Result<ExportableEncryptionKeyData, anyhow::Error> {
    let endpoint = format!("{}/{}", server, NCRYPTF_EK_ENDPOINT);

    let client = get_reqwest_client();
    let ek: ExportableEncryptionKeyData = client
        .get(endpoint)
        .send().await?
        .json::<ExportableEncryptionKeyData>().await?;

    Ok(ek)
}

pub(crate) fn get_reqwest_client() -> reqwest::Client {
    let mut builder = reqwest::Client
        ::builder()
        .use_rustls_tls()
        .timeout(Duration::new(5, 0))
        .danger_accept_invalid_certs(false);

    // In debug builds, allow invalid or bad certificates for testing
    #[cfg(dev)]
    {
    builder = builder.danger_accept_invalid_certs(true);
    }

    return builder.build().unwrap();
}
