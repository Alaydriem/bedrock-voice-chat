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
