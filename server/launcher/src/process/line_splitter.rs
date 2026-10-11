/// Splits a byte stream into `\n`-terminated lines, holding a partial line until it ends.
#[derive(Default)]
pub struct LineSplitter {
    partial: Vec<u8>,
}

impl LineSplitter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut lines = Vec::new();
        for &byte in bytes {
            self.partial.push(byte);
            if byte == b'\n' {
                lines.push(std::mem::take(&mut self.partial));
            }
        }
        lines
    }

    pub fn finish(&mut self) -> Option<Vec<u8>> {
        (!self.partial.is_empty()).then(|| std::mem::take(&mut self.partial))
    }
}
