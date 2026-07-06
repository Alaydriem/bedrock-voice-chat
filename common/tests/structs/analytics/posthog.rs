use chrono::TimeZone;
use common::structs::analytics::posthog::{BatchRequest, CaptureEvent};
use serde::Serialize;

#[derive(Serialize)]
struct TestProps {
    game: String,
}

// Guards the custom `serialize_with` on `CaptureEvent::timestamp`: PostHog requires
// an RFC3339 string, not chrono's default serialization. This is our serializer
// logic, not a DTO round-trip.
#[test]
fn timestamp_serializes_as_rfc3339() {
    let ts = chrono::Utc.with_ymd_and_hms(2026, 7, 5, 12, 0, 0).unwrap();
    let req = BatchRequest {
        api_key: "phc_test".to_string(),
        batch: vec![CaptureEvent {
            event: "player_connected".to_string(),
            distinct_id: "server-abc".to_string(),
            timestamp: ts,
            properties: TestProps {
                game: "minecraft".to_string(),
            },
        }],
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("2026-07-05T12:00:00+00:00"));
}
