//! Trust for the play zone (`*.play.bedrockvc.stream`).
//!
//! A BVC server in the play zone presents a certificate that chains to a BVC play root.
//! That root is name-constrained to the zone and is never added to a general root store.
//! [`PlayZoneVerifier`] is where the two halves of that rule are enforced on the client:
//! a play root can vouch for a play name and nothing else, and a play name is vouched for
//! by a play root and nothing else.

mod client_tls;
#[cfg(debug_assertions)]
mod permissive_verifier;
mod roots;
mod verifier;
mod verifier_mode;

pub use client_tls::BvcClientTls;
#[cfg(debug_assertions)]
pub use permissive_verifier::PermissiveVerifier;
pub use roots::PlayRoots;
pub use verifier::PlayZoneVerifier;
pub use verifier_mode::VerifierMode;
