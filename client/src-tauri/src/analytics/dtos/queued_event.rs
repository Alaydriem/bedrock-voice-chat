use common::structs::{AnalyticsEvent, AnalyticsEventData};

pub struct QueuedEvent {
    pub event: AnalyticsEvent,
    pub properties: Option<AnalyticsEventData>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
