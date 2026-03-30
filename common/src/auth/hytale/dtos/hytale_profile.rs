use serde::Deserialize;

/// Individual Hytale player profile
#[derive(Deserialize)]
pub(crate) struct HytaleProfile {
    pub uuid: String,
    pub username: String,
}
