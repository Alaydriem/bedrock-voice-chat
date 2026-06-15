pub(super) struct ParseResult {
    pub frames: Vec<Vec<u8>>,
    pub frame_count: usize,
    pub duration_ms: u64,
}
