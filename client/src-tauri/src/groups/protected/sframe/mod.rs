mod opened_frame;
mod sender_chain;

pub use opened_frame::OpenedFrame;
pub use sender_chain::SenderChain;

use std::time::{Duration, Instant};

use common::errors::SframeError;
use sframe::CipherSuite;
use sframe::error::SframeError as CrateError;
use sframe::frame::validation::{ReplayAttackProtectionStore, Tolerance};
use sframe::frame::{EncryptedFrame, MonotonicCounter};
use sframe::ratchet::{
    Generation, RatchetBits, RatchetStepDiff, RatchetingDecryptionKey, RatchetingEncryptionKey,
    RatchetingKeyId, RatchetingKeyStore,
};
use zeroize::Zeroizing;

/// Seals and opens a protected group's Opus frames (RFC 9605) with keys exported from MLS.
///
/// A key id is `generation | step`: the generation is `(sender leaf << 8) | (epoch & 0xFF)`,
/// which is the RFC 9605 § 5.2 MLS layout, and the low [`Self::RATCHET_BITS`] carry the § 5.1
/// ratchet step. Each sender steps its key forward every 64 frames and receivers follow, so a
/// device compromised mid-epoch exposes about a second of past audio rather than the epoch.
pub struct SframeCipher {
    own_leaf: u32,
    counter: MonotonicCounter,
    sender: Option<SenderChain>,
    receive: RatchetingKeyStore,
    replay: ReplayAttackProtectionStore,
    current: Vec<Generation>,
    retiring: Vec<(Generation, Instant)>,
}

impl SframeCipher {
    pub const SUITE: CipherSuite = CipherSuite::AesGcm256Sha512;
    pub const EPOCH_OVERLAP: Duration = Duration::from_secs(2);
    // 8 bits wrap after 255 steps, about 5.4 minutes of speech: longer than an epoch lives, so a
    // step number is never ambiguous within one.
    const RATCHET_BITS: u8 = 8;
    // How far a receiver catches up in one frame: 16 steps is about 20 seconds of lost audio.
    const MAX_CATCH_UP: u64 = 16;
    const REPLAY_TOLERANCE: usize = 128;

    pub fn new(own_leaf: u32) -> Self {
        Self {
            own_leaf,
            counter: MonotonicCounter::default(),
            sender: None,
            receive: RatchetingKeyStore::new(
                RatchetBits::new(Self::RATCHET_BITS),
                RatchetStepDiff::from(Self::MAX_CATCH_UP),
            ),
            replay: ReplayAttackProtectionStore::new(Tolerance::new(Self::REPLAY_TOLERANCE)),
            current: Vec::new(),
            retiring: Vec::new(),
        }
    }

    /// Installs one epoch's per-sender base keys. The previous epoch's receive keys stay usable
    /// for [`Self::EPOCH_OVERLAP`] so frames already in flight still open.
    pub fn install_epoch(&mut self, epoch: u64, base_keys: Vec<(u32, [u8; 32])>, now: Instant) {
        let deadline = now + Self::EPOCH_OVERLAP;
        self.retiring
            .extend(self.current.drain(..).map(|generation| (generation, deadline)));
        self.sender = None;

        for (leaf, base_key) in base_keys {
            let base_key = Zeroizing::new(base_key);
            let generation = Self::generation(leaf, epoch);
            let key_id = RatchetingKeyId::new(generation, RatchetBits::new(Self::RATCHET_BITS));
            if leaf == self.own_leaf {
                if let Ok(key) =
                    RatchetingEncryptionKey::derive_from(Self::SUITE, key_id, base_key.as_slice())
                {
                    self.sender = Some(SenderChain::new(key));
                }
                continue;
            }
            if let Ok(key) =
                RatchetingDecryptionKey::derive_from(Self::SUITE, key_id, base_key.as_slice())
            {
                self.receive.insert(key);
                self.current.push(generation);
            }
        }
    }

    pub fn seal(&mut self, opus: &[u8]) -> Result<Vec<u8>, SframeError> {
        let sender = self.sender.as_mut().ok_or(SframeError::NoEpoch)?;
        sender
            .seal(&mut self.counter, opus)
            .map_err(|_| SframeError::Authentication)
    }

    pub fn open(&mut self, sealed: &[u8], now: Instant) -> Result<OpenedFrame, SframeError> {
        self.expire(now);
        let frame = EncryptedFrame::try_from(sealed).map_err(|_| SframeError::Authentication)?;
        let key_id = frame.header().key_id();
        let opened = frame
            .validated_decrypt(&mut self.receive, &mut self.replay)
            .map_err(Self::classify)?;
        Ok(OpenedFrame {
            sender_leaf: Self::leaf_of(key_id),
            opus: opened.payload().to_vec(),
        })
    }

    /// Drops the previous epoch's receive keys once their overlap has passed.
    pub fn expire(&mut self, now: Instant) {
        let (expired, kept): (Vec<_>, Vec<_>) = self
            .retiring
            .drain(..)
            .partition(|(_, deadline)| now >= *deadline);
        self.retiring = kept;
        for (generation, _) in &expired {
            self.receive.remove(*generation);
        }
        if !expired.is_empty() {
            let gone: Vec<u64> = expired.iter().map(|(g, _)| u64::from(*g)).collect();
            self.replay
                .retain(|key_id| !gone.contains(&(u64::from(key_id) >> Self::RATCHET_BITS)));
        }
    }

    /// Drops every key. The crate zeroizes key material on drop.
    pub fn clear(&mut self) {
        *self = Self::new(self.own_leaf);
    }

    fn generation(leaf: u32, epoch: u64) -> Generation {
        Generation::from((u64::from(leaf) << 8) | (epoch & 0xFF))
    }

    fn leaf_of(key_id: u64) -> u32 {
        ((key_id >> Self::RATCHET_BITS) >> 8) as u32
    }

    fn classify(error: CrateError) -> SframeError {
        match error {
            CrateError::MissingDecryptionKey { .. } => SframeError::UnknownKey,
            CrateError::FrameValidationFailed(_) => SframeError::Replay,
            _ => SframeError::Authentication,
        }
    }
}
