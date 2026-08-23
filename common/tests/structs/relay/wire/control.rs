use common::structs::relay::Capability;
use common::structs::relay::wire::WireVersion;
use common::structs::relay::wire::control::{
    Accept, AudioQuery, ControlFrame, Hello, Refuse, RefuseReason,
};

// Fixed encodings, not round-trips.
//
// Postcard is positional and encodes an enum variant as its index, so these byte
// strings are the contract a third-party bridge implements against. A field
// reordered or a variant inserted anywhere but the end changes them, and a
// round-trip through our own types would not notice.

#[test]
fn hello_encodes_to_its_pinned_bytes() {
    let frame = ControlFrame::Hello(Hello {
        versions: vec![WireVersion(1)],
        worlds: vec!["w".to_string()],
    });

    // ControlFrame::Hello = 0, one version, version 1, one world of one byte "w".
    assert_eq!(
        frame.encode().expect("encode"),
        vec![0x00, 0x01, 0x01, 0x01, 0x01, 0x77]
    );
}

// A dialer that hosts nothing the acceptor will carry is refused rather than
// left holding a link that drops every frame.
#[test]
fn no_shared_world_encodes_to_its_pinned_bytes() {
    let frame = ControlFrame::Refuse(Refuse {
        reason: RefuseReason::NoSharedWorld,
    });

    // ControlFrame::Refuse = 2, RefuseReason::NoSharedWorld = 3.
    assert_eq!(frame.encode().expect("encode"), vec![0x02, 0x03]);
}

#[test]
fn accept_encodes_to_its_pinned_bytes() {
    let frame = ControlFrame::Accept(Accept {
        version: WireVersion(1),
        worlds: vec!["w".to_string()],
        capabilities: vec![Capability::CarrySpeakers],
    });

    // ControlFrame::Accept = 1, version 1, one world of one byte "w",
    // one capability, CarrySpeakers = 0.
    assert_eq!(
        frame.encode().expect("encode"),
        vec![0x01, 0x01, 0x01, 0x01, 0x77, 0x01, 0x00]
    );
}

#[test]
fn refuse_encodes_to_its_pinned_bytes() {
    let frame = ControlFrame::Refuse(Refuse {
        reason: RefuseReason::NoCommonVersion,
    });

    // ControlFrame::Refuse = 2, RefuseReason::NoCommonVersion = 0.
    assert_eq!(frame.encode().expect("encode"), vec![0x02, 0x00]);
}

#[test]
fn audio_query_encodes_to_its_pinned_bytes() {
    let frame = ControlFrame::AudioQuery(AudioQuery {
        audio_id: "a".to_string(),
        correlation_id: "c".to_string(),
    });

    // ControlFrame::AudioQuery = 3, then each string as length then bytes.
    assert_eq!(
        frame.encode().expect("encode"),
        vec![0x03, 0x01, 0x61, 0x01, 0x63]
    );
}

#[test]
fn a_peers_bytes_decode_to_the_frame_it_meant() {
    let decoded = ControlFrame::decode(&[0x02, 0x01]).expect("decode");

    match decoded {
        ControlFrame::Refuse(refuse) => {
            assert_eq!(refuse.reason, RefuseReason::NotAuthorized);
        }
        other => panic!("expected a Refuse frame, got {other:?}"),
    }
}

#[test]
fn a_truncated_frame_is_an_error_rather_than_a_panic() {
    // A Hello that promises one version and then ends.
    let result = ControlFrame::decode(&[0x00, 0x01]);

    assert!(
        result.is_err(),
        "a frame cut short must be reported, not silently completed"
    );
}

#[test]
fn an_unknown_variant_index_is_an_error() {
    let result = ControlFrame::decode(&[0x7F]);

    assert!(
        result.is_err(),
        "a variant this build does not know must be refused"
    );
}

#[test]
fn empty_input_is_an_error() {
    assert!(ControlFrame::decode(&[]).is_err());
}
