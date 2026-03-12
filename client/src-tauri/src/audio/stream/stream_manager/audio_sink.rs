use rodio::Player;
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
    Normal(Arc<Player>),
    Spatial(Arc<Player>),
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

    pub fn set_volume(&self, volume: f32) {
        match self {
            AudioSink::Normal(sink) => sink.set_volume(volume),
            AudioSink::Spatial(sink) => sink.set_volume(volume),
        }
    }

    pub fn append<S>(&self, source: S)
    where
        S: rodio::Source + Send + 'static,
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
