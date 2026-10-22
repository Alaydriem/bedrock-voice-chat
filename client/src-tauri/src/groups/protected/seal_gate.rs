use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use tauri_plugin_curia::curia;

use super::{GateState, RosterMember, SframeCipher};

/// Where the audio threads meet a protected group's keys.
///
/// The key service installs each epoch here; the capture thread seals through it and the playback
/// path opens through it. While protected, a frame without keys is dropped, never sent in the
/// clear. A plain mutex rather than an async one, because the audio threads are not async.
pub struct SealGate {
    protected: AtomicBool,
    state: Mutex<Option<GateState>>,
    // The group session that owns the keys. A replaced session's late cleanup must not wipe, or
    // install over, the keys of the session that replaced it.
    session: AtomicU64,
    // Mismatched frames are counted and reported at most once per interval: the server chooses
    // how many to send, and a warning per frame would let it flood the log.
    mismatches: AtomicU64,
    mismatch_reported_at: Mutex<Option<Instant>>,
}

impl SealGate {
    pub fn new() -> Self {
        Self {
            protected: AtomicBool::new(false),
            state: Mutex::new(None),
            session: AtomicU64::new(0),
            mismatches: AtomicU64::new(0),
            mismatch_reported_at: Mutex::new(None),
        }
    }

    const MISMATCH_REPORT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

    fn report_mismatch(&self) {
        let count = self.mismatches.fetch_add(1, Ordering::Relaxed) + 1;
        let Ok(mut reported_at) = self.mismatch_reported_at.lock() else {
            return;
        };
        let due = reported_at.is_none_or(|at| at.elapsed() >= Self::MISMATCH_REPORT_INTERVAL);
        if due {
            *reported_at = Some(Instant::now());
            self.mismatches.store(0, Ordering::Relaxed);
            curia::warn!(
                "dropped sealed frames whose key belongs to a different member than their sender stamp",
                { frames: count }
            );
        }
    }

    /// Starts a new group session on this gate, dropping any keys an older one left behind.
    /// Returns the id every later install and clear must present.
    pub fn begin_session(&self) -> u64 {
        let session = self.session.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(mut guard) = self.state.lock() {
            *guard = None;
        }
        session
    }

    fn is_current(&self, session: u64) -> bool {
        self.session.load(Ordering::SeqCst) == session
    }

    pub fn set_protected(&self, protected: bool) {
        self.protected.store(protected, Ordering::SeqCst);
    }

    pub fn is_protected(&self) -> bool {
        self.protected.load(Ordering::SeqCst)
    }

    pub fn has_keys(&self) -> bool {
        self.state.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Installs one epoch's keys and roster. The cipher keeps the previous epoch briefly so frames
    /// already in flight still open.
    pub fn install_epoch(
        &self,
        session: u64,
        own_leaf: u32,
        epoch: u64,
        base_keys: Vec<(u32, [u8; 32])>,
        roster: &[RosterMember],
        now: Instant,
    ) {
        let Ok(mut guard) = self.state.lock() else {
            return;
        };
        if !self.is_current(session) {
            return;
        }
        let state = guard.get_or_insert_with(|| GateState {
            cipher: SframeCipher::new(own_leaf),
            roster: HashMap::new(),
        });
        state.cipher.install_epoch(epoch, base_keys, now);
        state.roster = roster
            .iter()
            .map(|member| (member.leaf, member.identity.to_string()))
            .collect();
    }

    /// Drops every key, whichever session owns them. The gate stays protected, so nothing is sent
    /// until keys return.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = None;
        }
    }

    /// Drops the keys only if `session` still owns them.
    pub fn clear_session(&self, session: u64) {
        if let Ok(mut guard) = self.state.lock() {
            if self.is_current(session) {
                *guard = None;
            }
        }
    }

    /// Whether the group's own roster names `identity`.
    pub fn is_member(&self, identity: &str) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .as_ref()
                    .map(|state| state.roster.values().any(|name| name == identity))
            })
            .unwrap_or(false)
    }

    /// `None` while protected without keys: the caller sends nothing.
    pub fn seal(&self, opus: &[u8]) -> Option<Vec<u8>> {
        let mut guard = self.state.lock().ok()?;
        guard.as_mut()?.cipher.seal(opus).ok()
    }

    /// Opens a sealed frame, refusing it unless the roster names `server_sender` as the leaf whose
    /// key sealed it.
    pub fn open(&self, sealed: &[u8], server_sender: &str) -> Option<Vec<u8>> {
        let mut guard = self.state.lock().ok()?;
        let state = guard.as_mut()?;
        let opened = state.cipher.open(sealed, Instant::now()).ok()?;
        let named = state.roster.get(&opened.sender_leaf)?;
        if named != server_sender {
            drop(guard);
            self.report_mismatch();
            return None;
        }
        Some(opened.opus)
    }
}

impl Default for SealGate {
    fn default() -> Self {
        Self::new()
    }
}
