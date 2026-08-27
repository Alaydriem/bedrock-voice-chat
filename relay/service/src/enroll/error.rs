#[derive(Debug, thiserror::Error)]
pub enum EnrollError {
    #[error("binding the enrollment endpoint: {0}")]
    Bind(String),
    #[error("enrollment transport: {0}")]
    Transport(String),
    #[error("expected {expected} on the enrollment stream")]
    Unexpected { expected: &'static str },
    #[error(transparent)]
    Wire(#[from] common::errors::PeerWireError),
    #[error(transparent)]
    Registry(#[from] crate::registry::RegistryError),
    #[error(transparent)]
    Dns(#[from] crate::dns::DnsError),
}
