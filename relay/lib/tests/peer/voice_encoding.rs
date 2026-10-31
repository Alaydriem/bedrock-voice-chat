use bvc_relay::peer::VoiceEncoding;
use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
use common::structs::relay::wire::{Datagram, WireVersion};
use common::{Coordinate, Game, GenericPlayer, Orientation, PlayerEnum};

fn frame() -> VoiceFrame {
    VoiceFrame {
        speaker: PlayerEnum::Generic(GenericPlayer {
            name: String::new(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            game: Game::Minecraft,
        }),
        sample_rate: 48000,
        opus: vec![0xAA],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

fn open() -> GroupScope {
    GroupScope {
        channel: "ABCD-1234".to_string(),
        members_only: false,
    }
}

fn members_only() -> GroupScope {
    GroupScope {
        channel: "ABCD-1234".to_string(),
        members_only: true,
    }
}

const V1: WireVersion = WireVersion(1);

#[test]
fn v1_plain_is_voice() {
    assert!(matches!(
        VoiceEncoding::encode(V1, frame(), None),
        Some(Datagram::Voice(_))
    ));
}

// An OPEN group's speech is also heard nearby, so a version 1 peer gets it as plain speech,
// which is what it got before the marker existed.
#[test]
fn v1_open_marker_is_stripped() {
    assert!(matches!(
        VoiceEncoding::encode(V1, frame(), Some(open())),
        Some(Datagram::Voice(_))
    ));
}

// A version 1 peer cannot read the marker and would route the frame by proximity, which is
// the leak members-only speech exists to prevent.
#[test]
fn v1_members_only_is_not_sent() {
    assert!(VoiceEncoding::encode(V1, frame(), Some(members_only())).is_none());
}

#[test]
fn v2_plain_is_voice() {
    assert!(matches!(
        VoiceEncoding::encode(WireVersion::GROUP_SCOPE, frame(), None),
        Some(Datagram::Voice(_))
    ));
}

#[test]
fn v2_marked_is_group_voice() {
    let Some(Datagram::GroupVoice { group, .. }) =
        VoiceEncoding::encode(WireVersion::GROUP_SCOPE, frame(), Some(members_only()))
    else {
        panic!("a marked frame on a version 2 link must carry its group");
    };
    assert_eq!(group, members_only());
}

// Version 1 peers keep decoding exactly what they decoded before version 2 existed.
#[test]
fn v1_bytes_are_unchanged() {
    // The pinned bytes of `common`'s `a_voice_datagram_encodes_to_its_pinned_bytes`.
    let expected: Vec<u8> = vec![
        0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0xF7, 0x02, 0x01, 0xAA, 0x00,
        0x01, 0x00,
    ];

    let encoded = VoiceEncoding::encode(V1, frame(), Some(open()))
        .expect("an OPEN frame is sent on version 1")
        .to_datagram()
        .expect("encode");
    assert_eq!(encoded, expected);
}
