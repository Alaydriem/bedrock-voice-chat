use spake2::{Ed25519Group, Identity, Password, Spake2};

use super::GroupPassword;

/// Starts one side of the symmetric SPAKE2 exchange, bound to one group.
///
/// Symmetric mode, so joiner and admitter run identical code. The identity input carries the
/// group id, so a run against one group yields nothing usable against another. Every start draws
/// fresh randomness, so two sessions on one admitter never share a key.
pub struct PakeContext;

impl PakeContext {
    const LABEL: &'static [u8] = b"bvc-admission-v1";

    pub fn start(group_id: &str, password: &GroupPassword) -> (Spake2<Ed25519Group>, Vec<u8>) {
        let mut identity = Self::LABEL.to_vec();
        identity.extend_from_slice(group_id.as_bytes());
        Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(password.as_bytes()),
            &Identity::new(&identity),
        )
    }
}
