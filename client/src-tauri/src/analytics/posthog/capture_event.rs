use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::analytics::posthog::CaptureEventProperties;

#[derive(Debug, Clone, Serialize)]
pub struct CaptureEvent {
    pub event: String,
    pub distinct_id: String,
    #[serde(serialize_with = "CaptureEvent::serialize_rfc3339")]
    pub timestamp: DateTime<Utc>,
    pub properties: CaptureEventProperties,
}

impl CaptureEvent {
    fn serialize_rfc3339<S: serde::Serializer>(
        dt: &DateTime<Utc>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.serialize_str(&dt.to_rfc3339())
    }
}
