use common::errors::AdmissionError;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

/// A protected group's password, held only in memory and zeroed on drop.
///
/// NFC-normalized so the same password typed on two keyboards is the same bytes, and never
/// trimmed: a space is a character the user chose. `Debug` is redacted, so it cannot reach a log.
pub struct GroupPassword {
    normalized: Zeroizing<String>,
}

impl std::fmt::Debug for GroupPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GroupPassword(<redacted>)")
    }
}

impl GroupPassword {
    pub const MIN_CHARS: usize = 6;

    pub fn new(raw: &str) -> Result<Self, AdmissionError> {
        let normalized = Zeroizing::new(raw.nfc().collect::<String>());
        if normalized.chars().count() < Self::MIN_CHARS {
            return Err(AdmissionError::PasswordTooShort);
        }
        Ok(Self { normalized })
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.normalized.as_bytes()
    }
}
