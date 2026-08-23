use std::time::{Duration, Instant};

use common::structs::audio::LevelSnapshot;

/// When a level snapshot is worth a webview message, and when it is not.
///
/// On Android every message the backend sends the webview is a unit of main-thread work —
/// dequeue, marshal a JavaScript string across JNI, evaluate it — competing with layout and
/// rendering on the one thread that does all three. Two emitters running fixed 100 ms timers
/// spent about twenty of those a second on a meter, and the meter was the first thing to starve
/// when the main thread got busy. Message count is the entire cost; payload size is nearly free.
///
/// So this sends on change rather than on a clock, and never sends silence twice.
pub struct LevelEmitPolicy {
    last_sent: LevelSnapshot,
    last_at: Option<Instant>,
    ever_sent: bool,
}

impl LevelEmitPolicy {
    /// The shortest gap between messages once someone is already speaking.
    ///
    /// Amplitude alone never buys a message sooner than this. Speech changes at syllable rate,
    /// which would otherwise put a message on the wire for every motion the client can carry
    /// itself — the meter eases between heights and dances continuously in between, so what
    /// this rate decides is how quickly a change of loudness is answered, not whether the meter
    /// moves.
    ///
    /// 250 ms is four a second at the very most, against the twenty a second two fixed-rate
    /// emitters used to spend, so there is a wide margin to the rate that was starving the
    /// Android main thread.
    pub const MIN_GAP: Duration = Duration::from_millis(250);

    /// How long a client may go without hearing anything while somebody is speaking.
    ///
    /// The client expires a speaking flag it has not heard about, so a meter cannot animate
    /// forever over a backend that has died. This is what stops that expiry firing during a
    /// long, steady sentence.
    pub const KEEPALIVE: Duration = Duration::from_millis(1_000);

    pub fn new() -> Self {
        Self {
            last_sent: LevelSnapshot::silent(),
            last_at: None,
            ever_sent: false,
        }
    }

    /// Decide, and record the decision.
    ///
    /// A `true` result means the caller must send `next` — the policy has already taken it as
    /// the new baseline, so discarding it would leave the client holding a state this thinks it
    /// has been told about.
    pub fn admit(&mut self, now: Instant, next: &LevelSnapshot) -> bool {
        let send = self.should_send(now, next);
        if send {
            self.last_sent = next.clone();
            self.last_at = Some(now);
            self.ever_sent = true;
        }
        send
    }

    fn should_send(&self, now: Instant, next: &LevelSnapshot) -> bool {
        let elapsed = self.last_at.map(|at| now.saturating_duration_since(at));

        // Somebody started or stopped. This is the one the user is watching for, so it is not
        // rate-limited: a meter that waits out a gap before acknowledging a press-to-talk reads
        // as a control that did not take.
        if Self::voices_changed(&self.last_sent, next) {
            return true;
        }

        // Nothing has ever been sent, so the client is holding defaults rather than anything
        // this told it. Silence still needs no message: rest is what the client already draws.
        if !self.ever_sent {
            return !next.is_silent();
        }

        if next.is_silent() {
            // Already told them, and a client at rest stays at rest without being reminded.
            return false;
        }

        match elapsed {
            Some(gap) if gap >= Self::KEEPALIVE => true,
            Some(gap) if gap >= Self::MIN_GAP => next.differs_from(&self.last_sent),
            Some(_) => false,
            None => true,
        }
    }

    /// Whether anybody's speaking flag flipped, ignoring loudness entirely.
    fn voices_changed(previous: &LevelSnapshot, next: &LevelSnapshot) -> bool {
        if previous.own.speaking != next.own.speaking {
            return true;
        }

        // A peer who vanished counts as having stopped: their meter has to be told, and nothing
        // else in the payload will say so.
        next.peers.iter().any(|(name, level)| {
            previous
                .peers
                .get(name)
                .is_none_or(|was| was.speaking != level.speaking)
        }) || previous.peers.iter().any(|(name, level)| {
            level.speaking && !next.peers.contains_key(name)
        })
    }
}

impl Default for LevelEmitPolicy {
    fn default() -> Self {
        Self::new()
    }
}
