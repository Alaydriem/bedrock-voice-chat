use common::net::NetTimeouts;
use common::response::ApiConfigResponse;
use common::rustls::BvcClientTls;

use crate::auth::ServerUrlPath;

/// Reads a server's `/api/config` without credentials.
///
/// The request goes through `BvcClientTls`, so a play-zone server is validated against the
/// built-in play roots. The webview's HTTP plugin builds its own client from the public roots
/// and cannot do that.
pub(crate) struct PublicServerConfigReader;

impl PublicServerConfigReader {
    const PATH: &'static str = "api/config";

    // `base_url` is a resolved `https://host:port`.
    pub(crate) async fn read(base_url: &str) -> Result<ApiConfigResponse, anyhow::Error> {
        let client = BvcClientTls::http_client_builder(None, None)?
            .timeout(NetTimeouts::HTTPS)
            .build()?;

        let response = client
            .get(ServerUrlPath::join(base_url, Self::PATH))
            .send()
            .await?;
        let status = response.status();
        if status != common::reqwest::StatusCode::OK {
            anyhow::bail!("config returned {}", status.as_u16());
        }

        Ok(response.json::<ApiConfigResponse>().await?)
    }
}
