use common::structs::control::ClientActionType;
use serde_json::json;

// These exact JSON shapes are the cross-language contract the STANDALONE BDS (TS)
// and Java (Kotlin) encoders target. Those projects do not consume `common`, so a
// rename/reshape here would silently desync them at runtime. This test pins the wire
// shape so the break surfaces at CI time instead.
#[test]
fn client_action_type_wire_shape_is_stable() {
    assert_eq!(
        serde_json::to_value(ClientActionType::SetMuted(true)).unwrap(),
        json!({ "SetMuted": true })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::SetDeafened(false)).unwrap(),
        json!({ "SetDeafened": false })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::SetRecording(true)).unwrap(),
        json!({ "SetRecording": true })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::SetVolume {
            target: "Steve".into(),
            volume: 0.5
        })
        .unwrap(),
        json!({ "SetVolume": { "target": "Steve", "volume": 0.5 } })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::SetHeard {
            target: "Alex".into(),
            muted: true
        })
        .unwrap(),
        json!({ "SetHeard": { "target": "Alex", "muted": true } })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::CreateGroup).unwrap(),
        json!("CreateGroup")
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::JoinGroup {
            channel: "abc".into()
        })
        .unwrap(),
        json!({ "JoinGroup": { "channel": "abc" } })
    );
    assert_eq!(
        serde_json::to_value(ClientActionType::LeaveGroup).unwrap(),
        json!("LeaveGroup")
    );
}
