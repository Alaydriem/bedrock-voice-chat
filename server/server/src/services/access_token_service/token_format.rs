use sha2::{Digest, Sha256};

/// The wire format of a game access token: `bvc_<id>_<secret>`.
///
/// The identifier travels inside the token so verification is a primary-key lookup rather
/// than a comparison against every live credential, and so the value has a shape secret
/// scanners recognise.
pub struct TokenFormat;

impl TokenFormat {
    pub const PREFIX: &'static str = "bvc_";

    const ID_LEN: usize = 8;
    const SECRET_LEN: usize = 32;
    const ALPHABET: &'static str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    /// Returns `(id, secret)`. The secret is never stored; only its hash is.
    pub fn mint() -> (String, String) {
        let alphabet: Vec<char> = Self::ALPHABET.chars().collect();
        let id = nanoid::nanoid!(Self::ID_LEN, &alphabet);
        let secret = nanoid::nanoid!(Self::SECRET_LEN, &alphabet);
        (id, secret)
    }

    pub fn compose(id: &str, secret: &str) -> String {
        format!("{}{}_{}", Self::PREFIX, id, secret)
    }

    /// Splits a presented value into its parts, or `None` when it is not this format.
    ///
    /// The alphabet excludes `_`, so the first separator after the prefix is the only one.
    pub fn parse(presented: &str) -> Option<(&str, &str)> {
        let rest = presented.strip_prefix(Self::PREFIX)?;
        let (id, secret) = rest.split_once('_')?;

        if id.len() != Self::ID_LEN || secret.len() != Self::SECRET_LEN {
            return None;
        }
        if !id.chars().all(|c| c.is_ascii_alphanumeric())
            || !secret.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return None;
        }

        Some((id, secret))
    }

    /// Lowercase hex SHA-256.
    ///
    /// SHA-256 rather than a password KDF: the secret is 32 characters over a 62-character
    /// alphabet, so there is no brute-force surface for a slow hash to protect, and this
    /// runs on the position relay path.
    pub fn hash(secret: &str) -> String {
        Sha256::digest(secret.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
