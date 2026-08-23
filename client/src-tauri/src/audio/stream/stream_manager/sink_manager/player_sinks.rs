use std::sync::Arc;

use crate::audio::stream::jitter_buffer::PanState;
use crate::audio::stream::stream_manager::audio_sink::AudioSink;

#[derive(Clone, Default)]
pub(super) struct PlayerSinks {
    pub(super) normal: Option<Arc<AudioSink>>,
    pub(super) spatial: Option<Arc<AudioSink>>,
    pub(super) normal_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
    pub(super) spatial_handle: Option<crate::audio::stream::jitter_buffer::JitterBufferHandle>,
    pub(super) spatial_pan_state: Option<Arc<PanState>>,
}
