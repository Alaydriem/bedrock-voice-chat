use rodio::{Sink, SpatialSink};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioSinkType {
    Spatial,
    Normal,
}

impl AudioSinkType {
    pub fn from_spatial(spatial: bool) -> Self {
        if spatial {
            AudioSinkType::Spatial
        } else {
            AudioSinkType::Normal
        }
    }
}

#[derive(Clone)]
pub(crate) enum AudioSink {
    Normal(Arc<Sink>),
    Spatial(Arc<SpatialSink>),
}

impl AudioSink {
    pub fn play(&self) {
        match self {
            AudioSink::Normal(sink) => sink.play(),
            AudioSink::Spatial(sink) => sink.play(),
        }
    }

    pub fn clear_and_stop(&self) {
        match self {
            AudioSink::Normal(sink) => {
                sink.clear();
                sink.stop();
            }
            AudioSink::Spatial(sink) => {
                sink.clear();
                sink.stop();
            }
        }
    }

    /// Update spatial positioning using Rodio's built-in methods
    pub fn update_spatial_position(
        &self,
        emitter_pos: [f32; 3],
        left_ear: [f32; 3],
        right_ear: [f32; 3],
        volume: f32,
    ) {
        match self {
            AudioSink::Spatial(sink) => {
                sink.set_emitter_position(emitter_pos);
                sink.set_left_ear_position(left_ear);
                sink.set_right_ear_position(right_ear);
                sink.set_volume(volume);
            }
            AudioSink::Normal(sink) => {
                // For non-spatial sinks, just set volume
                sink.set_volume(volume);
            }
        }
    }

    /// Append a source to the sink
    pub fn append<S>(&self, source: S)
    where
        S: rodio::Source<Item = f32> + Send + 'static,
    {
        match self {
            AudioSink::Normal(sink) => {
                sink.append(source);
            }
            AudioSink::Spatial(sink) => {
                sink.append(source);
            }
        }
    }
}
