use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CaptureEvent<P: Serialize> {
    pub event: String,
    pub distinct_id: String,
    #[serde(serialize_with = "serialize_rfc3339")]
    pub timestamp: DateTime<Utc>,
    pub properties: P,
}

// PostHog expects an RFC3339 string timestamp.
fn serialize_rfc3339<S: serde::Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&dt.to_rfc3339())
}
