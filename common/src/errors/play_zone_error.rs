/// A failure to build the play-zone TLS verifier or a client configuration around it.
#[derive(Debug, thiserror::Error)]
pub enum PlayZoneError {
    #[error("a play root could not be read: {0}")]
    PlayRoot(String),
    #[error("an additional root certificate could not be read: {0}")]
    ExtraRoot(String),
    #[cfg(all(debug_assertions, feature = "dev-trust"))]
    #[error("a development trust root could not be used: {0}")]
    DevRoot(String),
    #[error("the client identity could not be used: {0}")]
    Identity(String),
    #[error("the TLS configuration could not be built: {0}")]
    Tls(#[from] rustls::Error),
    #[error("the certificate verifier could not be built: {0}")]
    Verifier(#[from] rustls::client::VerifierBuilderError),
}
