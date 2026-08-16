use std::num::NonZero;
use std::time::Duration;

use rodio::Source;

/// Converts a mono Source to stereo by duplicating each sample to both L and R channels
pub(super) struct MonoToStereo<S>
where
    S: Source,
{
    pub(super) inner: S,
    pub(super) pending_sample: Option<f32>,
}

impl<S> MonoToStereo<S>
where
    S: Source,
{
    pub(super) fn new(source: S) -> Self {
        Self {
            inner: source,
            pending_sample: None,
        }
    }
}

impl<S> Iterator for MonoToStereo<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // If we have a pending sample, return it as the R channel
        if let Some(sample) = self.pending_sample.take() {
            return Some(sample);
        }

        // Get next sample from mono source
        if let Some(sample) = self.inner.next() {
            // Store it for R channel
            self.pending_sample = Some(sample);
            // Return it as L channel
            Some(sample)
        } else {
            None
        }
    }
}

impl<S> Source for MonoToStereo<S>
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
