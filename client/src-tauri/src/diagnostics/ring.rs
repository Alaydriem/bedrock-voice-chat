use std::collections::VecDeque;

use common::structs::metrics::LinkSample;

// Rolling link history, owned here rather than in the front end so it survives a panel being
// closed and reopened, and so the copyable report can carry a trend — "it was fine thirty
// seconds ago" is otherwise unanswerable.
#[derive(Debug)]
pub struct SampleRing {
    samples: VecDeque<LinkSample>,
}

impl Default for SampleRing {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleRing {
    // Five minutes at one sample per second. Sized to the rollup interval so the percentiles and
    // the accumulated counters in one event describe the same window; a shorter ring would have the
    // two halves of a single event covering different spans, which a consumer cannot reconcile.
    // Comfortably covers a 72-second sparkline as well.
    pub const CAPACITY: usize = 300;

    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(Self::CAPACITY),
        }
    }

    pub fn push(&mut self, sample: LinkSample) {
        if self.samples.len() == Self::CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn samples(&self) -> Vec<LinkSample> {
        self.samples.iter().copied().collect()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    // Nearest-rank percentile over the samples that actually carry a measurement. Returns
    // `None` rather than `Some(0)` when nothing has been measured, because a zero would
    // render as a perfect link.
    pub fn rtt_percentile(&self, percentile: f32) -> Option<u32> {
        let mut measured: Vec<u32> = self.samples.iter().filter_map(|s| s.rtt_ms).collect();
        if measured.is_empty() {
            return None;
        }
        measured.sort_unstable();

        let rank = (percentile / 100.0 * measured.len() as f32).ceil() as usize;
        let index = rank.saturating_sub(1).min(measured.len() - 1);
        Some(measured[index])
    }

    pub fn rtt_max(&self) -> Option<u32> {
        self.samples.iter().filter_map(|s| s.rtt_ms).max()
    }

}
