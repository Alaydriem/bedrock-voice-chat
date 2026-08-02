/// Which delivery branch of `route_audio_frame` an interaction came through.
/// `Any` is not a branch: it is the deduplicated figure across both, stored
/// explicitly because distinct-player counts do not sum across routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionRoute {
    Proximity,
    Channel,
    Any,
}

impl InteractionRoute {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Proximity => "proximity",
            Self::Channel => "channel",
            Self::Any => "any",
        }
    }
}
