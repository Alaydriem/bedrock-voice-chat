pub struct BedrockWorldId;

impl BedrockWorldId {
    /// Stable per-world identifier derived from fields available in the
    /// Bedrock `StartGamePacket`. Every BVC client connecting to the same
    /// Bedrock server sees identical inputs and therefore produces an
    /// identical digest, so two proxy-side clients agree on world identity
    /// even when no BDS-side mod is installed.
    ///
    /// External implementors (e.g. a BDS-side mod that wants to publish a
    /// matching `world_uuid`) MUST mirror the formula byte-for-byte:
    /// `blake3(format!("{seed}|{level_id}|{world_name}"))`, hex-encoded.
    /// The inline test below pins the digest so accidental drift fails CI.
    pub fn derive(seed: i64, level_id: &str, world_name: &str) -> String {
        let key = format!("{seed}|{level_id}|{world_name}");
        blake3::hash(key.as_bytes()).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_is_stable() {
        let actual = BedrockWorldId::derive(123, "level-id", "My World");
        assert_eq!(
            actual,
            "cba824f1284f220f787dc7f56a42fa03fb40f885c43412a2b7287f85238c882d",
        );
    }

    #[test]
    fn derive_is_deterministic() {
        assert_eq!(
            BedrockWorldId::derive(42, "abc", "Hello"),
            BedrockWorldId::derive(42, "abc", "Hello"),
        );
    }

    #[test]
    fn derive_diverges_on_any_field_change() {
        let a = BedrockWorldId::derive(1, "abc", "World");
        let b = BedrockWorldId::derive(2, "abc", "World");
        let c = BedrockWorldId::derive(1, "def", "World");
        let d = BedrockWorldId::derive(1, "abc", "Other");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }
}
