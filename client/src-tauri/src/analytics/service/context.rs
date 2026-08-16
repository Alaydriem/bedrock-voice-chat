use crate::analytics::PlayerIdentity;


#[derive(Default, Clone)]
pub(super) struct AnalyticsContext {
    pub(super) connected_server: Option<String>,
    pub(super) player: Option<PlayerIdentity>,
}
