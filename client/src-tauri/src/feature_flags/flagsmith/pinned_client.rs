use std::sync::Arc;

use common::tls::SpkiPinningVerifier;

// Builds the reqwest client used to reach the Flagsmith host, pinning its
// certificate by SPKI hash so TLS only completes against the expected key.
pub(crate) struct FlagsmithPinnedClient;

impl FlagsmithPinnedClient {
    // SHA-256(SubjectPublicKeyInfo DER), base64 — the pinned public key(s) for
    // the Flagsmith host. Multiple entries allow rotating the key with a backup
    // pin without shipping a client update.
    const SPKI_PINS: &[&str] = &["Ae0qBh6ONPl2sGUMgJiRJE9mrho9ehVfPBPv3kI5eEo="];

    pub(crate) fn build() -> reqwest::Client {
        let pins: Vec<String> = Self::SPKI_PINS.iter().map(|p| p.to_string()).collect();
        let verifier = Arc::new(SpkiPinningVerifier::new(&pins));
        let provider = verifier.provider();

        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("rustls supports the default protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        reqwest::Client::builder()
            .use_preconfigured_tls(config)
            .build()
            .expect("failed to build pinned reqwest client")
    }
}
