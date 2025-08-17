use std::sync::{Arc, Mutex};
use rodio::{Sink, SpatialSink};

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
    Sink {
        sink: Arc<Sink>,
        decoder: Arc<Mutex<opus::Decoder>>,
    },
    SpatialSink {
        sink: Arc<SpatialSink>,
        decoder: Arc<Mutex<opus::Decoder>>,
    },
}

#[derive(Clone)]
pub(crate) enum AudioSinkTarget {
    Normal(Arc<Sink>),
    Spatial(Arc<SpatialSink>),
}

impl AudioSink {
    pub fn new(target: AudioSinkTarget, decoder: opus::Decoder) -> Self {
        match target {
            AudioSinkTarget::Normal(sink) => Self::Sink {
                sink,
                decoder: Arc::new(Mutex::new(decoder)),
            },
            AudioSinkTarget::Spatial(sink) => Self::SpatialSink {
                sink,
                decoder: Arc::new(Mutex::new(decoder)),
            },
        }
    }

    pub fn play(&self) {
        match self {
            AudioSink::Sink { sink, .. } => sink.play(),
            AudioSink::SpatialSink { sink, .. } => sink.play(),
        }
    }

    pub fn clear_and_stop(&self) {
        match self {
            AudioSink::Sink { sink, .. } => {
                sink.clear();
                sink.stop();
            }
            AudioSink::SpatialSink { sink, .. } => {
                sink.clear();
                sink.stop();
            }
        }
    }
}
