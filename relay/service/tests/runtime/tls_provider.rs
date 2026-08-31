use bvc_relay_service::runtime::TlsProvider;

// Two crypto providers reach this binary, so rustls has no default and the first thing
// to build a TLS config panics. That is not only the HTTPS listener: `reqwest::Client`
// builds one during construction, so the Discord and Cloudflare clients hit it during
// startup, before anything has served a request.
//
// The guard is the provider being resolvable at all. Without the install, both this
// and every outbound client die on a message naming rustls rather than the caller.
#[test]
fn installing_makes_a_crypto_provider_resolvable() {
    TlsProvider::install();

    assert!(
        rustls::crypto::CryptoProvider::get_default().is_some(),
        "no process-level crypto provider; every TLS config in this binary would panic"
    );
}

// Called from more than one entry point, and whichever runs first wins. A second call
// that panicked would make the order they run in load-bearing.
#[test]
fn installing_twice_is_not_an_error() {
    TlsProvider::install();
    TlsProvider::install();
}

// The outbound clients, which are what actually failed at startup. Constructing one is
// the assertion: it panics rather than returning an error when no provider is set.
#[test]
fn an_outbound_client_can_be_built() {
    TlsProvider::install();

    reqwest::Client::builder()
        .build()
        .expect("an HTTPS client the registry can reach Discord and Cloudflare with");
}
