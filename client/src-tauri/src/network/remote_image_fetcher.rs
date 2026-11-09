use std::sync::{Arc, OnceLock};
use std::time::Duration;

use common::reqwest::{self, redirect::Policy};
use common::rustls::{BvcClientTls, VerifierMode};
use url::Url;

/// Fetches an image the webview wants to show: a server's avatar and canvas art, and
/// player portraits.
///
/// The request goes through `BvcClientTls` because the webview validates against the
/// operating system's roots, which do not include the play roots.
///
/// The webview's own `fetch` is bound by CORS and this is not, so the fetcher accepts only
/// http and https, follows a redirect only within the host it started at, and returns only
/// a body the server labels as an image and that fits `MAX_BYTES`.
pub struct RemoteImageFetcher {
    client: reqwest::Client,
}

impl RemoteImageFetcher {
    const TIMEOUT: Duration = Duration::from_secs(10);
    const MAX_REDIRECTS: usize = 5;

    /// The largest body the fetcher returns.
    pub const MAX_BYTES: usize = 10 * 1024 * 1024;

    pub fn new() -> Result<Self, anyhow::Error> {
        let client = BvcClientTls::http_client_builder(None, None, VerifierMode::for_build())?
            .timeout(Self::TIMEOUT)
            .redirect(Policy::custom(|attempt| {
                if attempt.previous().len() >= Self::MAX_REDIRECTS {
                    return attempt.error("too many redirects");
                }

                let origin = attempt.previous().first();
                let target = attempt.url();
                let same_host = origin.is_some_and(|origin| {
                    origin.host() == target.host()
                        && origin.port_or_known_default() == target.port_or_known_default()
                });
                let web_scheme = matches!(target.scheme(), "http" | "https");

                if same_host && web_scheme {
                    attempt.follow()
                } else {
                    attempt.error("the image redirects to another host")
                }
            }))
            .build()?;

        Ok(Self { client })
    }

    /// One fetcher for the process, so the TLS configuration and the play roots are built once.
    pub fn shared() -> Result<Arc<Self>, anyhow::Error> {
        static SHARED: OnceLock<Arc<RemoteImageFetcher>> = OnceLock::new();

        if let Some(fetcher) = SHARED.get() {
            return Ok(fetcher.clone());
        }

        let fetcher = Arc::new(Self::new()?);
        Ok(SHARED.get_or_init(|| fetcher).clone())
    }

    pub async fn fetch(&self, url: &str) -> Result<Vec<u8>, anyhow::Error> {
        let parsed = Url::parse(url)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            anyhow::bail!("only http and https images can be fetched");
        }

        let mut response = self.client.get(parsed).send().await?.error_for_status()?;

        let is_image = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.trim().to_ascii_lowercase().starts_with("image/"));
        if !is_image {
            anyhow::bail!("the response is not an image");
        }

        if response
            .content_length()
            .is_some_and(|length| length > Self::MAX_BYTES as u64)
        {
            anyhow::bail!("the image is larger than {} bytes", Self::MAX_BYTES);
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > Self::MAX_BYTES {
                anyhow::bail!("the image is larger than {} bytes", Self::MAX_BYTES);
            }
            body.extend_from_slice(&chunk);
        }

        Ok(body)
    }
}
