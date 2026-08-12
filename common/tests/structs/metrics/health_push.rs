use common::structs::metrics::{HealthPush, MetricsPush};
use common::structs::network::ConnectionHealth;

// The discriminant is the contract. A consumer reads both frame kinds off one socket, and
// `ResponseData` on the command protocol is `#[serde(untagged)]` — so without a distinct `type`
// a health frame would have to be told apart from a metrics frame by guessing from which fields
// happen to be present.
#[test]
fn a_health_frame_is_distinguishable_from_a_metrics_frame() {
    let push = HealthPush::new(ConnectionHealth::Reconnecting { attempt: 3 });
    let json = serde_json::to_value(&push).expect("health push serialises");

    assert_eq!(json["type"], "health");
    assert_ne!(HealthPush::KIND, MetricsPush::KIND);
}

// The reason a client is not connected is the whole payload. A frame that said only "not
// connected" would leave an overlay unable to separate a network drop from a refused identity,
// which are different problems with different remedies — one resolves itself, one never will.
#[test]
fn the_reason_survives_the_envelope() {
    let push = HealthPush::new(ConnectionHealth::Unauthorized {
        reason: "certificate expired".to_string(),
    });
    let json = serde_json::to_value(&push).expect("health push serialises");

    assert_eq!(json["data"]["status"], "Unauthorized");
    assert_eq!(json["data"]["reason"], "certificate expired");
}

// A reconnect attempt count is what an overlay counts down. Losing it would leave "reconnecting"
// indistinguishable from "reconnecting for the twentieth time", which is the difference between
// waiting and giving up.
#[test]
fn the_attempt_number_survives_the_envelope() {
    let push = HealthPush::new(ConnectionHealth::Reconnecting { attempt: 7 });
    let json = serde_json::to_value(&push).expect("health push serialises");

    assert_eq!(json["data"]["status"], "Reconnecting");
    assert_eq!(json["data"]["attempt"], 7);
}
