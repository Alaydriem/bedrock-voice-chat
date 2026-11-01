// The s2n-quic provider and the mTLS HTTP client need the `quic` and `net` features
// respectively. The play-zone verifier needs neither, so a build with only `net`
// (the default) still gets it.
#[cfg(feature = "net")]
pub mod http_client;
#[cfg(feature = "quic")]
mod mtls_provider;
pub mod play_zone;

#[cfg(feature = "net")]
pub use http_client::MtlsHttpClient;
#[cfg(feature = "quic")]
pub use mtls_provider::MtlsProvider;
pub use play_zone::{BvcClientTls, PlayRoots, PlayZoneVerifier, VerifierMode};
