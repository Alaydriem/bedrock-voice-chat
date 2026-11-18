/// One player as a game mod reports them to `bvc_update_positions`, including the identity
/// fields a Floodgate-aware Java mod sends.
#[derive(Debug, Clone)]
pub struct ReportedPosition {
    pub name: String,
    pub alternative_identity: Option<String>,
    pub player_uuid: Option<String>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ReportedPosition {
    pub fn new(name: &str, alternative_identity: Option<&str>, player_uuid: Option<&str>, x: f32) -> Self {
        Self {
            name: name.to_string(),
            alternative_identity: alternative_identity.map(String::from),
            player_uuid: player_uuid.map(String::from),
            x,
            y: 64.0,
            z: 0.0,
        }
    }
}
