use rand::RngExt;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::errors::PairingCodeError;

// A pairing code, held as the digest of the plaintext an operator types.
//
// The plaintext exists in two places: the terminal that minted it, and one frame on one
// connection. Everything durable holds this instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCode {
    digest: [u8; Self::DIGEST_BYTES],
}

impl PairingCode {
    pub const DIGEST_BYTES: usize = 32;

    // Eight characters of the alphabet below is a little over 40 bits. Wide enough that
    // the attempt counter, not the width, is what an attacker runs out of.
    pub const PLAINTEXT_LEN: usize = 8;

    // Crockford base32: no I, L, O or U. The excluded letters are the ones a person
    // reading a code off a screen substitutes for digits, and U is excluded so a
    // generated code cannot spell an unwelcome word.
    const ALPHABET: &'static [u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    pub fn generate() -> (String, Self) {
        let mut rng = rand::rng();

        let plaintext: String = (0..Self::PLAINTEXT_LEN)
            .map(|_| Self::ALPHABET[rng.random_range(0..Self::ALPHABET.len())] as char)
            .collect();

        let code = Self::from_plaintext(&plaintext);

        (plaintext, code)
    }

    pub fn from_plaintext(code: &str) -> Self {
        Self {
            digest: Self::hash(code),
        }
    }

    // Storage holds the hex digest rather than this type's serialized form: a column
    // carrying a serde envelope is unreadable by anything but this exact struct
    // definition, and the digest is the whole of the value.
    pub fn from_hex(hex: &str) -> Result<Self, PairingCodeError> {
        if hex.len() != Self::DIGEST_BYTES * 2 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PairingCodeError::Digest);
        }

        let mut digest = [0u8; Self::DIGEST_BYTES];

        for (index, byte) in digest.iter_mut().enumerate() {
            let pair = &hex[index * 2..index * 2 + 2];
            *byte = u8::from_str_radix(pair, 16).map_err(|_| PairingCodeError::Digest)?;
        }

        Ok(Self { digest })
    }

    pub fn to_hex(&self) -> String {
        self.digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    // Uppercased, stripped of the separators a person adds, and with the characters the
    // alphabet excludes folded onto the digits they are mistaken for.
    pub fn normalize(raw: &str) -> String {
        raw.chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| match c.to_ascii_uppercase() {
                'I' | 'L' => '1',
                'O' => '0',
                other => other,
            })
            .collect()
    }

    // Constant time. The code is a secret, and a timing-variable compare on a secret is
    // a defect regardless of how remote the attack is. Hashing first also keeps the
    // comparison length-independent, so the secret's length does not leak.
    pub fn verify(&self, candidate: &str) -> bool {
        let candidate = Self::hash(candidate);

        self.digest.ct_eq(&candidate).into()
    }

    fn hash(code: &str) -> [u8; Self::DIGEST_BYTES] {
        let mut hasher = Sha256::new();
        hasher.update(Self::normalize(code).as_bytes());

        hasher.finalize().into()
    }
}
