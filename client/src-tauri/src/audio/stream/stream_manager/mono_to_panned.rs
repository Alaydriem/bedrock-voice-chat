use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;

use crate::audio::stream::jitter_buffer::PanState;

// ~4.2ms time constant at 48kHz
const SMOOTH_COEFF: f32 = 0.005;

pub(crate) struct MonoToPanned<S>
where
    S: Source,
{
    inner: S,
    pan_state: Arc<PanState>,
    pending_right: Option<f32>,
    current_left: f32,
    current_right: f32,
    current_volume: f32,
}

impl<S> MonoToPanned<S>
where
    S: Source,
{
    pub fn new(source: S, pan_state: Arc<PanState>) -> Self {
        let initial_left = pan_state.left_gain();
        let initial_right = pan_state.right_gain();
        let initial_volume = pan_state.volume();
        Self {
            inner: source,
            pan_state,
            pending_right: None,
            current_left: initial_left,
            current_right: initial_right,
            current_volume: initial_volume,
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
            let target_left = self.pan_state.left_gain();
            let target_right = self.pan_state.right_gain();
            let target_volume = self.pan_state.volume();

            self.current_left += (target_left - self.current_left) * SMOOTH_COEFF;
            self.current_right += (target_right - self.current_right) * SMOOTH_COEFF;
            self.current_volume += (target_volume - self.current_volume) * SMOOTH_COEFF;

            self.pending_right = Some(sample * self.current_volume * self.current_right);
            Some(sample * self.current_volume * self.current_left)
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
