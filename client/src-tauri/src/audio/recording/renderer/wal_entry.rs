use common::structs::recording::RecordingHeader;

/// Raw WAL entry containing Opus packet and metadata
#[derive(Debug)]
pub struct WalEntry {
    pub header: RecordingHeader,
    pub opus_data: Vec<u8>,
    pub relative_timestamp_ms: u64,
}
