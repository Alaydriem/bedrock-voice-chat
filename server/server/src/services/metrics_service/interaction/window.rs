use dashmap::DashMap;

use super::counts::InteractionCounts;

/// One route's view of the window currently being measured. Entries carry the
/// window they were written in, so a survivor of a concurrent clear is ignored
/// rather than leaking into the next window's count.
pub struct InteractionWindow {
    pairs: DashMap<u128, u64>,
    participants: DashMap<u64, u64>,
    mutual: DashMap<u64, u64>,
}

impl InteractionWindow {
    pub fn new() -> Self {
        Self {
            pairs: DashMap::new(),
            participants: DashMap::new(),
            mutual: DashMap::new(),
        }
    }

    /// Records one delivered frame. This runs once per *recipient*, not once per
    /// frame, so at a fanout of N it fires N times for every serialization the
    /// route does. Everything past the first delivery of a pair in a window costs
    /// one sharded read lock and an integer compare — the audio path cannot afford
    /// a set insert per frame.
    ///
    /// The read-then-insert is deliberately not atomic. Two threads racing the
    /// same pair both proceed, and every write they then make is an idempotent
    /// insert into a distinct-key map, so the counts are unaffected. `mutual` is
    /// safe for the same reason plus ordering: each thread inserts its own pair
    /// before reading the reverse, so both directions cannot miss each other.
    pub fn record(&self, pair: u128, sender: u64, recipient: u64, window: u64) {
        if self.pairs.get(&pair).is_some_and(|w| *w == window) {
            return;
        }
        self.pairs.insert(pair, window);
        self.participants.insert(sender, window);
        self.participants.insert(recipient, window);

        let reverse = ((recipient as u128) << 64) | sender as u128;
        if self.pairs.get(&reverse).is_some_and(|w| *w == window) {
            self.mutual.insert(sender, window);
            self.mutual.insert(recipient, window);
        }
    }

    pub fn counts(&self, window: u64) -> InteractionCounts {
        InteractionCounts {
            reached: Self::live(&self.participants, window),
            mutual: Self::live(&self.mutual, window),
        }
    }

    // `pairs` is cleared last, and that order is load-bearing. It is the dedup gate:
    // a delivery landing mid-clear that inserts into an already-cleared `pairs` but
    // has its participants wiped afterwards would be early-returned forever after,
    // leaving both players uncounted for the whole window. Clearing the gate last
    // means any such stray self-heals on the pair's next frame.
    pub fn clear(&self) {
        self.participants.clear();
        self.mutual.clear();
        self.pairs.clear();
    }

    fn live(map: &DashMap<u64, u64>, window: u64) -> u64 {
        map.iter().filter(|e| *e.value() == window).count() as u64
    }
}

impl Default for InteractionWindow {
    fn default() -> Self {
        Self::new()
    }
}
