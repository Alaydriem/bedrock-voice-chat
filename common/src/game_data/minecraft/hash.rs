use serde::{Deserialize, Serialize};

/// The identity of a dimension BVC has no variant for.
///
/// The first 8 bytes of blake3 over the canonical name. Fixed in size whatever name a mod
/// sends, so a player record and a datagram carry the same cost for any dimension. The
/// formula is compared across versions, relay peers and bridges, and must not change.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DimensionHash([u8; 8]);

// Hex, the same text the `custom:<hex>` wire form carries, so a mismatch in an error or
// debug log can be matched to the dimension behind it.
impl std::fmt::Debug for DimensionHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DimensionHash({})", self.to_hex())
    }
}

impl DimensionHash {
    const HEX_LEN: usize = 16;

    pub fn of(canonical: &str) -> Self {
        let digest = blake3::hash(canonical.as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest.as_bytes()[..8]);
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        format!("{:016x}", u64::from_be_bytes(self.0))
    }

    /// Exactly 16 lowercase hex characters, or `None`. Uppercase is refused so a hash has
    /// one written form.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let lowercase_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if hex.len() != Self::HEX_LEN || !hex.chars().all(lowercase_hex) {
            return None;
        }

        u64::from_str_radix(hex, 16)
            .ok()
            .map(|value| Self(value.to_be_bytes()))
    }
}
