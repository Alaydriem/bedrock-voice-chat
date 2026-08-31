// The process-wide rustls precondition.
//
// Both aws-lc-rs and ring reach this binary through the dependency graph, and rustls
// refuses to choose between them: with two compiled in there is no default provider,
// and the first thing to build a TLS config panics instead of failing.
//
// That first thing is not the HTTPS listener. `reqwest::Client::new()` builds one too,
// so the Discord and Cloudflare clients trip it during startup — before anything has
// served a request, and with a message that names rustls rather than the caller.
pub struct TlsProvider;

impl TlsProvider {
    // aws-lc-rs, to match the iroh endpoint's pin and the BVC server.
    //
    // Idempotent: already-installed is not an error, so the first caller wins and
    // nothing here overrides a choice made earlier. Every entry point may call it, and
    // one that needs TLS should.
    pub fn install() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}
