use std::sync::{LazyLock, Mutex};

use super::bridge::WireCapture;

static CAPTURE: LazyLock<Mutex<WireCapture>> = LazyLock::new(|| Mutex::new(WireCapture::default()));

/// Records what this client sends at the boundaries the server can observe.
///
/// Fed from `e2e`-gated calls at each boundary, so the record is the bytes as they left rather
/// than a reconstruction of them.
pub struct WireTap;

impl WireTap {
    // Per kind, so a long scenario cannot grow the record without bound. Far above what any
    // capture window holds: fifty frames a second for over six minutes.
    const MAX_ENTRIES: usize = 20_000;

    pub fn datagram(bytes: &[u8]) {
        Self::record(|capture| &mut capture.datagrams, bytes);
    }

    pub fn key_frame(bytes: &[u8]) {
        Self::record(|capture| &mut capture.key_frames, bytes);
    }

    pub fn http_body(bytes: &[u8]) {
        Self::record(|capture| &mut capture.http_bodies, bytes);
    }

    pub fn sealed_plaintext(bytes: &[u8]) {
        Self::record(|capture| &mut capture.sealed_plaintext, bytes);
    }

    /// Everything recorded since the last call, leaving the record empty.
    pub fn take() -> WireCapture {
        CAPTURE
            .lock()
            .map(|mut capture| std::mem::take(&mut *capture))
            .unwrap_or_default()
    }

    fn record(kind: impl FnOnce(&mut WireCapture) -> &mut Vec<Vec<u8>>, bytes: &[u8]) {
        if let Ok(mut capture) = CAPTURE.lock() {
            let entries = kind(&mut capture);
            if entries.len() < Self::MAX_ENTRIES {
                entries.push(bytes.to_vec());
            }
        }
    }
}
