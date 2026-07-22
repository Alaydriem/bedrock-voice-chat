use common::structs::control::{ClientActionType, CtlCodec, CtlMessage};

fn decode_action(name: &str) -> ClientActionType {
    match CtlCodec::decode(name) {
        Some(CtlMessage::Action(a)) => a,
        _ => panic!("expected action from {name}"),
    }
}

#[test]
fn encodes_and_decodes_each_action_round_trip() {
    let cases = vec![
        ClientActionType::SetMuted(true),
        ClientActionType::SetDeafened(false),
        ClientActionType::SetRecording(true),
        ClientActionType::SetVolume {
            target: "Steve".into(),
            volume: 0.5,
        },
        ClientActionType::SetHeard {
            target: "Alex".into(),
            muted: true,
        },
        ClientActionType::CreateGroup,
        ClientActionType::JoinGroup {
            channel: "abc123".into(),
        },
        ClientActionType::LeaveGroup,
    ];
    for a in cases {
        let encoded = CtlCodec::encode(&a);
        assert_eq!(decode_action(&encoded), a, "round-trip failed for {encoded}");
    }
}

#[test]
fn decodes_known_wire_strings() {
    assert_eq!(
        decode_action("bvc:ctl:mute:1"),
        ClientActionType::SetMuted(true)
    );
    assert_eq!(
        decode_action("bvc:ctl:vol:Steve:70"),
        ClientActionType::SetVolume {
            target: "Steve".into(),
            volume: 0.70
        }
    );
    assert_eq!(
        decode_action("bvc:ctl:group:join:abc123"),
        ClientActionType::JoinGroup {
            channel: "abc123".into()
        }
    );
    assert_eq!(
        decode_action("bvc:ctl:group:leave"),
        ClientActionType::LeaveGroup
    );
}

#[test]
fn hear_flag_inverts_to_muted() {
    // "hear on" (flag 1) means NOT muted; "hear off" (flag 0) means muted.
    assert_eq!(
        decode_action("bvc:ctl:hear:Alex:1"),
        ClientActionType::SetHeard {
            target: "Alex".into(),
            muted: false
        }
    );
    assert_eq!(
        decode_action("bvc:ctl:hear:Alex:0"),
        ClientActionType::SetHeard {
            target: "Alex".into(),
            muted: true
        }
    );
}

#[test]
fn decodes_sync_with_target_list() {
    match CtlCodec::decode("bvc:ctl:sync:Steve,Alex") {
        Some(CtlMessage::Sync { targets }) => {
            assert_eq!(targets, vec!["Steve".to_string(), "Alex".to_string()]);
        }
        _ => panic!("expected sync"),
    }
}

#[test]
fn rejects_non_ctl_and_unknown_verbs() {
    assert!(CtlCodec::decode("bvc:play:x:minecraft:overworld").is_none());
    assert!(CtlCodec::decode("bvc:ctl:bogus").is_none());
    assert!(CtlCodec::decode("bvc:ctl:group:frobnicate").is_none());
}
