pub(crate) mod login;

use reqwest::Client;
use std::time::Duration;

/// Returns an Reqwest Client configured to talk to BVC API
pub(crate) fn get_reqwest_client() -> Client {
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::new(5, 0))
        .danger_accept_invalid_certs(false);

    // In debug builds, allow invalid or bad certificates for testing
    #[cfg(debug_assertions)]
    {
        builder = builder.danger_accept_invalid_certs(true);
    }

    return builder.build().unwrap();
}
