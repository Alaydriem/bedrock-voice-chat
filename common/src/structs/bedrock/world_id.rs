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
    /// An integration test pins the digest so accidental drift fails CI.
    pub fn derive(seed: i64, level_id: &str, world_name: &str) -> String {
        let key = format!("{seed}|{level_id}|{world_name}");
        blake3::hash(key.as_bytes()).to_hex().to_string()
    }
}
