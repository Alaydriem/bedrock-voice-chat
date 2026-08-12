use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;

use crate::audio::spatial::{GainSmoother, SpatialGains};
use crate::audio::stream::jitter_buffer::PanState;

pub(crate) struct MonoToPanned<S>
where
    S: Source,
{
    inner: S,
    pan_state: Arc<PanState>,
    pending_right: Option<f32>,
    smoother: GainSmoother,
}

impl<S> MonoToPanned<S>
where
    S: Source,
{
    pub fn new(source: S, pan_state: Arc<PanState>) -> Self {
        let initial = SpatialGains {
            left: pan_state.left_gain(),
            right: pan_state.right_gain(),
            volume: pan_state.volume(),
        };
        Self {
            inner: source,
            pan_state,
            pending_right: None,
            smoother: GainSmoother::new(initial),
        }
    }
}

impl<S> Iterator for MonoToPanned<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.pending_right.take() {
            return Some(sample);
        }

        if let Some(sample) = self.inner.next() {
            let target = SpatialGains {
                left: self.pan_state.left_gain(),
                right: self.pan_state.right_gain(),
                volume: self.pan_state.volume(),
            };
            let current = self.smoother.advance(&target);

            self.pending_right = Some(sample * current.volume * current.right);
            Some(sample * current.volume * current.left)
        } else {
            None
        }
    }
}

impl<S> Source for MonoToPanned<S>
where
    S: Source,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len().map(|len| len * 2)
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}
