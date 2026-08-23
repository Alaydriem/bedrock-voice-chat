/// How a player identity becomes a WAL segment filename, and back again.
///
/// The identity carries a colon, which NTFS reads as an alternate-data-stream separator,
/// so the key written to disk is a stripped and truncated form of it. Reading a session
/// back has to apply exactly the same rule, which is why one type owns both directions.
pub struct WalKey;

impl WalKey {
    const MAX_LEN: usize = 20;
    const SEGMENT_SUFFIX: &'static str = ".log";

    pub fn sanitize(key: &str) -> String {
        key.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(Self::MAX_LEN)
            .collect()
    }

    /// Segments are named `SanitizedKey-hash-sequence.log`. The separator is load-bearing:
    /// without it a short key selects every longer key that starts with it.
    pub fn matches(filename: &str, key: &str) -> bool {
        filename.starts_with(&format!("{}-", Self::sanitize(key)))
            && filename.ends_with(Self::SEGMENT_SUFFIX)
    }
}
