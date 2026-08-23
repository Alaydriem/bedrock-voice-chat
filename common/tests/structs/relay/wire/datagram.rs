use common::structs::packet::MAX_DATAGRAM_SIZE;
use common::structs::relay::wire::Datagram;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, Game, GenericPlayer, Orientation, PlayerEnum};

// A speaker whose every numeric field is zero, so the pinned encoding below is
// short enough to read and contains no float bit patterns to transcribe.
fn speaker() -> PlayerEnum {
    PlayerEnum::Generic(GenericPlayer {
        name: String::new(),
        coordinates: Coordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        game: Game::Minecraft,
    })
}

fn frame() -> Datagram {
    Datagram::Voice(VoiceFrame {
        speaker: speaker(),
        sample_rate: 48000,
        opus: vec![0xAA],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    })
}

#[test]
fn a_voice_datagram_encodes_to_its_pinned_bytes() {
    let expected: Vec<u8> = vec![
        // Datagram::Voice
        0x00, // PlayerEnum::Generic
        0x02, // name, zero length
        0x00, // coordinates x, y, z
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // orientation x, y
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Game::Minecraft
        0x00, // sample_rate 48000
        0x80, 0xF7, 0x02, // opus, one byte
        0x01, 0xAA, // timestamp_ms 0
        0x00, // spatial true
        0x01, // jukebox absent
        0x00,
    ];

    assert_eq!(frame().to_datagram().expect("encode"), expected);
}

// Speech is the common case and pays a single byte for a field it never uses.
#[test]
fn an_absent_jukebox_costs_one_byte() {
    let with_field = frame().to_datagram().expect("encode");

    assert_eq!(
        with_field.last().copied(),
        Some(0x00),
        "an absent jukebox must encode as one zero byte: {with_field:?}"
    );
}

// The event id is what keeps concurrent playbacks on separate sinks at the far
// end, so it has to survive the round trip intact rather than merely be present.
#[test]
fn a_jukebox_event_id_survives_the_round_trip() {
    let encoded = Datagram::Voice(VoiceFrame {
        speaker: speaker(),
        sample_rate: 48000,
        opus: vec![0xAA],
        timestamp_ms: 0,
        spatial: true,
        jukebox: Some("evt-1234".to_string()),
    })
    .to_datagram()
    .expect("encode");

    let Datagram::Voice(decoded) = Datagram::from_datagram(&encoded).expect("decode");

    assert_eq!(decoded.jukebox, Some("evt-1234".to_string()));
}

#[test]
fn a_datagram_over_the_cap_is_refused_rather_than_sent() {
    let oversized = Datagram::Voice(VoiceFrame {
        speaker: speaker(),
        sample_rate: 48000,
        opus: vec![0u8; MAX_DATAGRAM_SIZE],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    });

    assert!(
        oversized.to_datagram().is_err(),
        "a frame larger than the cap must fail loudly at the encoder"
    );
}

#[test]
fn an_inbound_datagram_over_the_cap_is_refused_before_decoding() {
    let too_long = vec![0u8; MAX_DATAGRAM_SIZE + 1];

    assert!(
        Datagram::from_datagram(&too_long).is_err(),
        "an oversized datagram must be refused without being parsed"
    );
}

#[test]
fn a_frame_at_the_cap_is_accepted() {
    // The largest opus payload that still fits, found by shrinking until it does.
    let mut payload = MAX_DATAGRAM_SIZE;
    let encoded = loop {
        let candidate = Datagram::Voice(VoiceFrame {
            speaker: speaker(),
            sample_rate: 48000,
            opus: vec![0u8; payload],
            timestamp_ms: 0,
            spatial: true,
            jukebox: None,
        });
        match candidate.to_datagram() {
            Ok(bytes) => break bytes,
            Err(_) => payload -= 1,
        }
    };

    assert!(encoded.len() <= MAX_DATAGRAM_SIZE);
    assert!(
        Datagram::from_datagram(&encoded).is_ok(),
        "a datagram this build produced must be one it accepts"
    );
}

#[test]
fn a_truncated_datagram_is_an_error_rather_than_a_panic() {
    let encoded = frame().to_datagram().expect("encode");
    let truncated = &encoded[..encoded.len() - 2];

    assert!(Datagram::from_datagram(truncated).is_err());
}
