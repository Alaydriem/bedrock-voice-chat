use rodio::mixer::Mixer;
use std::sync::Arc;

// Resolved mix target shared by every output sink variant: the mixer the
// SinkManager feeds decoded/jittered/spatialized sources into, plus (cpal only)
// the live device sink that must stay resident to keep draining the mixer. The
// fake variant spawns its own drain thread instead of returning one, so playback
// has one body that simply consumes `mixer` regardless of variant.
pub(crate) struct MixTarget {
    pub mixer: Arc<Mixer>,
    pub playback_stream: Option<rodio::MixerDeviceSink>,
}
