use std::net::SocketAddr;

use common::structs::relay::enroll::EnrollRefuseReason;

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("too many play names claimed")]
    RateLimited,
    #[error("this node holds no play name")]
    NoPlayName,
    #[error("the certificate request was refused: {0}")]
    CsrRejected(String),
    #[error("{0} is not a global unicast address")]
    AddressRefused(std::net::IpAddr),
    #[error("{0} is behind carrier-grade NAT")]
    AddressCgnat(std::net::IpAddr),
    #[error("signing the certificate failed: {0}")]
    Signer(String),
    #[error("binding the play DNS server on {address}: {source}")]
    Bind {
        address: SocketAddr,
        source: std::io::Error,
    },
    #[error("play DNS: {0}")]
    Dns(String),
    #[error(transparent)]
    Naming(#[from] crate::naming::NamingError),
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}

impl PlayError {
    // What the far side is told. Coarser than the local error.
    pub fn refuse_reason(&self) -> EnrollRefuseReason {
        match self {
            Self::RateLimited => EnrollRefuseReason::RateLimited,
            Self::NoPlayName => EnrollRefuseReason::NoPlayName,
            Self::CsrRejected(_) => EnrollRefuseReason::CsrRejected,
            Self::AddressRefused(_) => EnrollRefuseReason::AddressNotPublic,
            Self::AddressCgnat(_) => EnrollRefuseReason::AddressCgnat,
            _ => EnrollRefuseReason::Internal,
        }
    }
}
