use std::time::Duration;

use common::tls::LetsEncryptRootStore;

// Builds the reqwest client used to reach the Flagsmith host, trusting only
// Let's Encrypt's ISRG roots so TLS completes only against a certificate that
// chains up to them.
pub(crate) struct FlagsmithPinnedClient;

impl FlagsmithPinnedClient {
    // reqwest applies no timeout by default, so a blackholed route to the
    // Flagsmith host would stall the fetch indefinitely. Every flag read waits
    // on the first fetch completing, so an unbounded fetch is an unbounded UI
    // stall. Capped so a slow Flagsmith degrades to each flag's default.
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

    pub(crate) fn build() -> reqwest::Client {
        reqwest::Client::builder()
            .use_preconfigured_tls(LetsEncryptRootStore::client_config())
            .timeout(Self::REQUEST_TIMEOUT)
            .connect_timeout(Self::CONNECT_TIMEOUT)
            .build()
            .expect("failed to build pinned reqwest client")
    }
}
