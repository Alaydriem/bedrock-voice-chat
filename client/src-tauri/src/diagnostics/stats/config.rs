use std::sync::Mutex;

// Server-supplied voice settings, recorded where they are resolved so a diagnostic can report
// what this session is actually running under rather than what the defaults say.
//
// Separate from `LinkSession` because these arrive from the audio pipeline while the connection
// facts arrive from the transport, at different moments.
#[derive(Debug, Default)]
pub struct SessionConfig {
    spatial: Mutex<Option<Spatial>>,
}

#[derive(Debug, Clone)]
struct Spatial {
    proximity_range: f32,
    falloff: String,
}

impl SessionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_spatial(&self, proximity_range: f32, falloff: impl Into<String>) {
        if let Ok(mut guard) = self.spatial.lock() {
            *guard = Some(Spatial {
                proximity_range,
                falloff: falloff.into(),
            });
        }
    }

    pub fn proximity_range(&self) -> Option<f32> {
        self.spatial
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.proximity_range))
    }

    pub fn falloff(&self) -> Option<String> {
        self.spatial
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.falloff.clone()))
    }
}
