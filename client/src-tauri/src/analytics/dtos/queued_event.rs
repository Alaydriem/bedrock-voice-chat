use common::structs::{AnalyticsEvent, AnalyticsEventData};

pub struct QueuedEvent {
    pub event: AnalyticsEvent,
    pub properties: Option<AnalyticsEventData>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub connected_server: Option<String>,
    pub player_display: Option<String>,
    pub player_hash: Option<String>,
}
