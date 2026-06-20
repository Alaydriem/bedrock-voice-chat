use anyhow::anyhow;
use dryoc::dryocbox::{DryocBox, PublicKey};

// Minter-side sealing: seal a code to a recipient's advertised public key.
pub struct CodeSealer;

impl CodeSealer {
    // Seals `code` to `recipient_pubkey` (32-byte X25519), returning a hex-encoded
    // sealed box safe to carry as a whitespace-free realm chat token.
    pub fn seal(code: &str, recipient_pubkey: &[u8]) -> Result<String, anyhow::Error> {
        let pk = PublicKey::try_from(recipient_pubkey)
            .map_err(|_| anyhow!("invalid recipient public key length"))?;
        let boxed = DryocBox::seal_to_vecbox(code.as_bytes(), &pk)
            .map_err(|e| anyhow!("seal failed: {e}"))?;
        Ok(hex::encode(boxed.to_vec()))
    }
}
