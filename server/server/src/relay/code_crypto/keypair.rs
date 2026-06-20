use std::sync::Arc;

use dryoc::dryocbox::{DryocBox, KeyPair};
use dryoc::types::Bytes;

use crate::relay::observe::CodeDecryptor;

// The asker's X25519 keypair for sealed-box code delivery. The asker advertises
// its public key in `/api/relay/offer`; the minter seals the minted code to it,
// and only the asker (holding the secret key) can unseal the ciphertext that
// travels through the realm. This makes the realm-delivered code unreadable to any
// observer — the cryptographic half of recipient-binding.
pub struct RelayCodeKeypair {
    keypair: KeyPair,
}

impl Default for RelayCodeKeypair {
    fn default() -> Self {
        Self::new()
    }
}

impl RelayCodeKeypair {
    pub fn new() -> Self {
        Self {
            keypair: KeyPair::r#gen(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // The asker's public key bytes, advertised in the offer for the minter to seal
    // the code to.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.keypair.public_key.as_slice().to_vec()
    }

    // Unseals a hex-encoded sealed box observed in the realm back to the plaintext
    // code. Returns `None` when the ciphertext is malformed or was not sealed to
    // this keypair's public key (i.e. not ours to open).
    pub fn unseal(&self, hex_ciphertext: &str) -> Option<String> {
        let bytes = hex::decode(hex_ciphertext).ok()?;
        let boxed = DryocBox::from_sealed_bytes(&bytes).ok()?;
        let plain = boxed.unseal_to_vec(&self.keypair).ok()?;
        String::from_utf8(plain).ok()
    }
}

impl CodeDecryptor for RelayCodeKeypair {
    fn decrypt(&self, observed: &str) -> Option<String> {
        self.unseal(observed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::code_crypto::sealer::CodeSealer;

    // Security invariant: a code sealed to one asker cannot be opened by anyone
    // else — an observer who intercepts the realm ciphertext cannot read it.
    #[test]
    fn a_non_recipient_cannot_open_the_sealed_code() {
        let kp_a = RelayCodeKeypair::new();
        let kp_b = RelayCodeKeypair::new();
        let sealed = CodeSealer::seal("CODE", &kp_a.public_key_bytes()).expect("seal");
        assert_eq!(kp_b.unseal(&sealed), None);
        // The bound recipient still opens it (positive control for the guard).
        assert_eq!(kp_a.unseal(&sealed).as_deref(), Some("CODE"));
    }
}
