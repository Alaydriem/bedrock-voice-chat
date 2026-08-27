// Everything needed to open a session, as one record so the generated Kotlin
// takes a single named argument rather than four positional ones.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SdkConfig {
    pub node_dir: String,
    pub peerlink: String,
    pub worlds: Vec<String>,
    // A handful of frames. Larger delivers stale audio late, and the consumer
    // has its own buffering — two in series is two sets of latency.
    pub inbox_capacity: u32,
}
