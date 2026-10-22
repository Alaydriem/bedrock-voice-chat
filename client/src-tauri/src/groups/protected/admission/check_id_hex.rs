/// A check id as it crosses into the webview and back: exactly 32 lower-case hex characters.
pub struct CheckIdHex;

impl CheckIdHex {
    pub fn encode(check_id: &[u8; 16]) -> String {
        check_id.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// `None` unless the text is exactly 32 hex characters.
    pub fn decode(text: &str) -> Option<[u8; 16]> {
        if text.len() != 32 || !text.is_ascii() {
            return None;
        }
        let mut out = [0u8; 16];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(out)
    }
}
