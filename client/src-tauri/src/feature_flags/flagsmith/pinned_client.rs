use common::tls::LetsEncryptRootStore;

// Builds the reqwest client used to reach the Flagsmith host, trusting only
// Let's Encrypt's ISRG roots so TLS completes only against a certificate that
// chains up to them.
pub(crate) struct FlagsmithPinnedClient;

impl FlagsmithPinnedClient {
    pub(crate) fn build() -> reqwest::Client {
        reqwest::Client::builder()
            .use_preconfigured_tls(LetsEncryptRootStore::client_config())
            .build()
            .expect("failed to build pinned reqwest client")
    }
}
