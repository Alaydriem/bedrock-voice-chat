// Why a validation pass could not reach a verdict.
//
// Distinct from `RegistryError` because a verdict needs both halves — the
// registration and the zone — and making the registry aware of DNS to share one
// error type would invert the layering: the registry decides who holds a name, and
// the zone is one of the things that acts on that decision.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error(transparent)]
    Registry(#[from] crate::registry::RegistryError),
    #[error(transparent)]
    Dns(#[from] crate::dns::DnsError),
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}
