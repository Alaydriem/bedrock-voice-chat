/// Caps text that came from a client before it is logged or kept as a cache key, so a
/// large value in a request cannot become a large log line or a large key.
pub struct BoundedLogText;

impl BoundedLogText {
    pub const MAX_CHARS: usize = 128;

    pub fn of(text: &str) -> String {
        text.chars().take(Self::MAX_CHARS).collect()
    }
}
