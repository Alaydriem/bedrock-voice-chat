use common::structs::control::{BvcsCodec, BvcsMessage, PlayerPreference, QueryState};

// The `!bvcs:` grammar is a wire contract with the standalone BDS mod's decoder
// (mods/bds/src/state/bvcs_codec.ts). These goldens pin the exact strings; a
// drifting encoder breaks the in-game panel silently.

#[test]
fn query_state_encodes_golden_wire_string() {
    let state = QueryState {
        id: "Alice".into(),
        muted: true,
        deafened: false,
        recording: false,
        current_group: Some("abc123".into()),
    };
    assert_eq!(
        BvcsCodec::encode_query_state(7, &state),
        "!bvcs:7:q:m=1;d=0;r=0;g=abc123"
    );

    let no_group = QueryState {
        current_group: None,
        ..state
    };
    assert_eq!(
        BvcsCodec::encode_query_state(8, &no_group),
        "!bvcs:8:q:m=1;d=0;r=0;g=-"
    );
}

#[test]
fn preference_encodes_golden_wire_string_with_heard_flag_and_percent_volume() {
    let pref = PlayerPreference {
        owner: "Alice".into(),
        target: "Bob".into(),
        volume: 0.5,
        muted: true,
    };
    // muted=true rides as h=0 (heard flag), volume as a rounded percent —
    // both mirror the bvc:ctl: grammar's conventions.
    assert_eq!(
        BvcsCodec::encode_preference(9, &pref),
        "!bvcs:9:p:t=Bob;v=50;h=0"
    );
}

#[test]
fn query_state_round_trips_through_decode() {
    let state = QueryState {
        id: "Alice".into(),
        muted: false,
        deafened: true,
        recording: true,
        current_group: None,
    };
    let encoded = BvcsCodec::encode_query_state(3, &state);
    match BvcsCodec::decode(&encoded) {
        Some(BvcsMessage::QueryState {
            muted,
            deafened,
            recording,
            group,
        }) => {
            assert!(!muted);
            assert!(deafened);
            assert!(recording);
            assert_eq!(group, None);
        }
        _ => panic!("expected QueryState from {encoded}"),
    }
}

#[test]
fn preference_round_trips_through_decode() {
    let pref = PlayerPreference {
        owner: "Alice".into(),
        target: "Bob".into(),
        volume: 0.25,
        muted: false,
    };
    let encoded = BvcsCodec::encode_preference(4, &pref);
    match BvcsCodec::decode(&encoded) {
        Some(BvcsMessage::Preference {
            target,
            volume,
            muted,
        }) => {
            assert_eq!(target, "Bob");
            assert!((volume - 0.25).abs() < 1e-6);
            assert!(!muted);
        }
        _ => panic!("expected Preference from {encoded}"),
    }
}

#[test]
fn rejects_non_bvcs_and_malformed_messages() {
    assert!(BvcsCodec::decode("hello world").is_none());
    assert!(BvcsCodec::decode("!bvcp token").is_none());
    assert!(BvcsCodec::decode("!bvcs:").is_none());
    assert!(BvcsCodec::decode("!bvcs:1:x:m=1").is_none());
    assert!(BvcsCodec::decode("!bvcs:1:q:").is_none());
    assert!(BvcsCodec::decode("!bvcs:1:p:t=Bob").is_none());
    assert!(BvcsCodec::decode("!bvcs:nan:q:m=1;d=0;r=0;g=-").is_none());
}

#[test]
fn rejects_non_finite_preference_volumes() {
    assert!(BvcsCodec::decode("!bvcs:1:p:t=Bob;v=nan;h=1").is_none());
    assert!(BvcsCodec::decode("!bvcs:1:p:t=Bob;v=inf;h=1").is_none());
}

#[test]
fn delimiter_bearing_targets_are_not_wire_safe() {
    // A target carrying grammar delimiters could inject or override sibling
    // fields (e.g. "x;v=999" shadows the real volume); the encoder's callers
    // must skip such targets until the grammar gains escaping.
    assert!(BvcsCodec::target_is_wire_safe("Gamer Tag 42"));
    assert!(!BvcsCodec::target_is_wire_safe("x;v=999"));
    assert!(!BvcsCodec::target_is_wire_safe("a=b"));
    assert!(!BvcsCodec::target_is_wire_safe("a:b"));
}

#[test]
fn the_group_list_header_is_pinned() {
    assert_eq!(BvcsCodec::encode_group_header(7, 3), "!bvcs:7:gl:c=3");
    assert_eq!(
        BvcsCodec::decode("!bvcs:7:gl:c=3"),
        Some(BvcsMessage::GroupListHeader { count: 3 })
    );
}

#[test]
fn an_empty_server_rides_a_zero_header() {
    // c=0 is how an empty server says so. Riding nothing would be indistinguishable
    // from the proxy never answering, which the panel reports differently.
    assert_eq!(
        BvcsCodec::decode("!bvcs:1:gl:c=0"),
        Some(BvcsMessage::GroupListHeader { count: 0 })
    );
}

#[test]
fn a_group_row_is_pinned() {
    assert_eq!(
        BvcsCodec::encode_group(8, "Xk3_9aQ", "Sculk Striders"),
        "!bvcs:8:g:i=Xk3_9aQ;n=Sculk Striders"
    );
    assert_eq!(
        BvcsCodec::decode("!bvcs:8:g:i=Xk3_9aQ;n=Sculk Striders"),
        Some(BvcsMessage::Group {
            id: "Xk3_9aQ".to_string(),
            name: "Sculk Striders".to_string(),
        })
    );
}

#[test]
fn a_name_carrying_grammar_characters_survives_the_round_trip() {
    // A renamed group can carry anything a player can type. Dropping such a row would
    // hide the group from everyone rather than showing it with an awkward name.
    let name = "a;b=c:d%e";
    let encoded = BvcsCodec::encode_group(9, "id1", name);
    assert_eq!(encoded, "!bvcs:9:g:i=id1;n=a%3Bb%3Dc%3Ad%25e");
    assert_eq!(
        BvcsCodec::decode(&encoded),
        Some(BvcsMessage::Group {
            id: "id1".to_string(),
            name: name.to_string(),
        })
    );
}

#[test]
fn a_row_whose_name_cannot_be_decoded_is_dropped() {
    assert_eq!(BvcsCodec::decode("!bvcs:9:g:i=id1;n=a%ZZ"), None);
    assert_eq!(BvcsCodec::decode("!bvcs:9:g:i=id1;n=trailing%"), None);
}
