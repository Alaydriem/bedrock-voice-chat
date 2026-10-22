use sframe::frame::{MediaFrame, MonotonicCounter};
use sframe::ratchet::RatchetingEncryptionKey;

/// This member's own sending key for the current epoch, stepping forward every
/// [`SenderChain::RATCHET_FRAMES`] frames.
///
/// The crate zeroizes key material when a key is dropped, so stepping past a key, or dropping the
/// chain, leaves nothing behind that could open earlier frames.
pub struct SenderChain {
    key: RatchetingEncryptionKey,
    frames_in_step: u64,
}

impl SenderChain {
    pub const RATCHET_FRAMES: u64 = 64;

    pub fn new(key: RatchetingEncryptionKey) -> Self {
        Self {
            key,
            frames_in_step: 0,
        }
    }

    /// Seals one frame. The counter is the caller's, so it stays monotonic across epochs and a
    /// nonce is never reused even if two epochs derived related keys.
    pub fn seal(
        &mut self,
        counter: &mut MonotonicCounter,
        opus: &[u8],
    ) -> Result<Vec<u8>, sframe::error::SframeError> {
        if self.frames_in_step == Self::RATCHET_FRAMES {
            self.key = self.key.ratchet()?;
            self.frames_in_step = 0;
        }
        self.frames_in_step += 1;
        let encrypted = MediaFrame::try_new(counter, opus)?.encrypt(self.key.as_ref())?;
        Ok(encrypted.as_ref().to_vec())
    }
}
