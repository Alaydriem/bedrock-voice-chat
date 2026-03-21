use serde::Serialize;
use chrono::{DateTime, Utc};
use crate::analytics::aptabase::SystemProps;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(serialize_with = "Event::serialize_rfc3339")]
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_name: String,
    pub system_props: SystemProps,
    pub props: serde_json::Value,
}

impl Event {
    fn serialize_rfc3339<S: serde::Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&dt.to_rfc3339())
    }
}
