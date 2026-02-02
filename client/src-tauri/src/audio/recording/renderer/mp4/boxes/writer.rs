//! Generic binary writer for MP4 boxes
//!
//! Provides a chainable API for constructing MP4 boxes with proper big-endian encoding.

/// A chainable binary writer for constructing MP4 boxes
#[derive(Debug, Clone)]
pub struct BoxWriter {
    buffer: Vec<u8>,
}

impl BoxWriter {
    /// Create a new empty BoxWriter
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Create a new BoxWriter with pre-allocated capacity
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(cap),
        }
    }

    /// Write a single byte
    pub fn u8(mut self, val: u8) -> Self {
        self.buffer.push(val);
        self
    }

    /// Write a big-endian u16
    pub fn u16(mut self, val: u16) -> Self {
        self.buffer.extend_from_slice(&val.to_be_bytes());
        self
    }

    /// Write a big-endian i16
    pub fn i16(mut self, val: i16) -> Self {
        self.buffer.extend_from_slice(&val.to_be_bytes());
        self
    }

    /// Write a big-endian u32
    pub fn u32(mut self, val: u32) -> Self {
        self.buffer.extend_from_slice(&val.to_be_bytes());
        self
    }

    /// Write a big-endian u64
    pub fn u64(mut self, val: u64) -> Self {
        self.buffer.extend_from_slice(&val.to_be_bytes());
        self
    }

    /// Write a 4-character code (fourcc)
    pub fn fourcc(mut self, code: &[u8; 4]) -> Self {
        self.buffer.extend_from_slice(code);
        self
    }

    /// Write zero bytes
    pub fn zeros(mut self, count: usize) -> Self {
        self.buffer.extend(std::iter::repeat(0u8).take(count));
        self
    }

    /// Write raw bytes
    pub fn bytes(mut self, data: &[u8]) -> Self {
        self.buffer.extend_from_slice(data);
        self
    }

    /// Write a complete box with size prefix and fourcc type
    ///
    /// The size includes the 8-byte header (4 bytes size + 4 bytes type)
    pub fn write_box(mut self, box_type: &[u8; 4], content: &[u8]) -> Self {
        let size = 8 + content.len() as u32;
        self.buffer.extend_from_slice(&size.to_be_bytes());
        self.buffer.extend_from_slice(box_type);
        self.buffer.extend_from_slice(content);
        self
    }

    /// Append another BoxWriter's contents
    pub fn append(mut self, other: &BoxWriter) -> Self {
        self.buffer.extend_from_slice(&other.buffer);
        self
    }

    /// Get current length of the buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Get reference to the internal buffer
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume the writer and return the built bytes
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }
}

impl Default for BoxWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_writes() {
        let result = BoxWriter::new()
            .u8(0xFF)
            .u16(0x1234)
            .u32(0xDEADBEEF)
            .finish();

        assert_eq!(result, vec![0xFF, 0x12, 0x34, 0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_fourcc() {
        let result = BoxWriter::new().fourcc(b"moov").finish();
        assert_eq!(result, b"moov".to_vec());
    }

    #[test]
    fn test_write_box() {
        let result = BoxWriter::new()
            .write_box(b"test", &[0x01, 0x02, 0x03])
            .finish();

        // Size = 8 (header) + 3 (content) = 11 = 0x0000000B
        assert_eq!(
            result,
            vec![0x00, 0x00, 0x00, 0x0B, b't', b'e', b's', b't', 0x01, 0x02, 0x03]
        );
    }

    #[test]
    fn test_zeros() {
        let result = BoxWriter::new().zeros(4).finish();
        assert_eq!(result, vec![0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_chaining() {
        let result = BoxWriter::new()
            .u32(1)
            .fourcc(b"test")
            .zeros(2)
            .bytes(&[0xAB, 0xCD])
            .finish();

        assert_eq!(
            result,
            vec![0x00, 0x00, 0x00, 0x01, b't', b'e', b's', b't', 0x00, 0x00, 0xAB, 0xCD]
        );
    }
}
