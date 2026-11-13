//! Trust for the play zone (`*.play.bedrockvc.stream`).
//!
//! A BVC server in the play zone presents a certificate that chains to a BVC play root.
//! That root is name-constrained to the zone and is never added to a general root store.
//! [`PlayZoneVerifier`] is where the two halves of that rule are enforced on the client:
//! a play root can vouch for a play name and nothing else, and a play name is vouched for
//! by a play root and nothing else.

mod client_tls;
#[cfg(all(debug_assertions, feature = "dev-trust"))]
mod dev_trust_roots;
mod roots;
mod verifier;

pub use client_tls::BvcClientTls;
#[cfg(all(debug_assertions, feature = "dev-trust"))]
pub use dev_trust_roots::DevTrustRoots;
pub use roots::PlayRoots;
pub use verifier::PlayZoneVerifier;
