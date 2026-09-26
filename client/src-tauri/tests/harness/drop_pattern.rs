/// Which downstream datagrams a `LossyUdpRelay` discards, by ordinal.
///
/// `OneIn` is the original fixture: exact rate, never two in a row. `Burst` is what a saturated
/// uplink actually does — several consecutive losses — and is the only shape that exercises a
/// jitter buffer's gap handling rather than its single-frame concealment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPattern {
    OneIn(u64),
    Burst { every: u64, consecutive: u64 },
}

impl DropPattern {
    /// `seen` is 1-based: the first datagram observed is 1.
    pub fn should_drop(&self, seen: u64) -> bool {
        match *self {
            DropPattern::OneIn(n) => seen % n == 0,
            // Ordinals `every - consecutive` through `every - 1` of each window, counted modulo
            // `every`. Anchored at the top of the window rather than its start, so the first
            // datagrams of a session are never dropped and the handshake completes.
            DropPattern::Burst { every, consecutive } => (seen % every) >= every - consecutive,
        }
    }
}

#[test]
fn one_in_drops_exactly_the_nth() {
    let p = DropPattern::OneIn(10);
    let dropped: Vec<u64> = (1..=30).filter(|s| p.should_drop(*s)).collect();
    assert_eq!(dropped, vec![10, 20, 30]);
}

#[test]
fn burst_drops_consecutive_ordinals_at_the_end_of_each_window() {
    let p = DropPattern::Burst {
        every: 50,
        consecutive: 2,
    };
    let dropped: Vec<u64> = (1..=100).filter(|s| p.should_drop(*s)).collect();
    assert_eq!(dropped, vec![48, 49, 98, 99]);
}

#[test]
fn burst_never_drops_the_first_datagrams_of_a_session() {
    let p = DropPattern::Burst {
        every: 50,
        consecutive: 2,
    };
    assert!((1..=47).all(|s| !p.should_drop(s)));
}
