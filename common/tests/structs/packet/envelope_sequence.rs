use common::structs::packet::{
    AudioFramePacket, HealthCheckPacket, MAX_DATAGRAM_SIZE, PacketSender, PacketType,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use common::structs::packet::SpeakerPosition;
use common::{Coordinate, Game, PlayerIdentity};
use serde::{Deserialize, Serialize};

// Version skew for the 3.0.0 envelope, established by measurement rather than assumption.
//
// The headline result: the envelope's shape is a BREAKING protocol change in both directions.
// Postcard is not self-describing and its format is positional, so `#[serde(default)]` does nothing
// for a missing field — a decoder reads the bytes in declaration order and mis-parses or runs off
// the end when the order it expects is not the order it receives. A field addition is additive
// under a self-describing format; it is not under this one.
//
// `reference_versioned_codec_zero_packet` in this repo is the standing reminder that codec-level
// assumptions get verified. These tests are that verification, and they contradicted the assumption.

// The envelope as a 2.1.0 peer defines it: an identity the client claimed for itself, and no
// sequence. Declared locally because both types it named are deleted — that is the point of the
// version being 3.0.0.
#[derive(Serialize, Deserialize)]
struct LegacyOwner {
    name: String,
    client_id: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct LegacyEnvelope {
    packet_type: PacketType,
    owner: Option<LegacyOwner>,
    data: QuicNetworkPacketData,
}

fn identity(gamertag: &str) -> PlayerIdentity {
    Game::Minecraft.membership_key(gamertag)
}

fn legacy() -> LegacyEnvelope {
    LegacyEnvelope {
        packet_type: PacketType::HealthCheck,
        owner: None,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
    }
}

fn current() -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::HealthCheck,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        ..Default::default()
    }
}

#[test]
fn a_new_client_cannot_decode_an_old_servers_datagram() {
    // A 3.0.0 decoder reads the old `owner` discriminant as the start of `data`, tags the packet as
    // whichever variant that byte happens to name, and then runs off the end looking for its fields.
    //
    // This is why the shape change requires a protocol version bump rather than riding as an
    // addition. It is asserted rather than merely noted so that a future attempt to make any of
    // these fields "optional" fails here instead of in the field.
    let bytes = postcard::to_stdvec(&legacy()).expect("legacy encodes");

    assert!(
        QuicNetworkPacket::from_datagram(&bytes).is_err(),
        "a new client decoding old bytes must fail loudly at the codec, not silently mis-parse"
    );
}

#[test]
fn an_old_client_cannot_decode_a_new_servers_datagram() {
    // The break is symmetric, and that is an improvement worth pinning. While the envelope merely
    // gained a trailing `seq`, this direction survived — postcard ignores trailing bytes — and
    // asymmetric compatibility is worse than none, because an old client keeps working against a new
    // server right up until it is itself updated, hiding the break for the length of a rollout.
    //
    // Removing `owner` closed that window: the old decoder now reads `data`'s variant tag where it
    // expects an `Option` discriminant and rejects it. Both sides fail at the codec on the first
    // datagram, which is where a version mismatch should be discovered.
    let bytes = current().to_datagram().expect("encodes");

    assert!(
        postcard::from_bytes::<LegacyEnvelope>(&bytes).is_err(),
        "an old client decoding new bytes must fail at the codec rather than half-working"
    );
}

#[test]
fn a_stamped_envelope_round_trips_at_the_widest_encoding() {
    let mut packet = current();
    packet.stamp(u32::MAX);

    let bytes = packet.to_datagram().expect("encodes");
    let decoded = QuicNetworkPacket::from_datagram(&bytes).expect("decodes");

    assert_eq!(decoded.sequence(), Some(u32::MAX));
}

#[test]
fn an_unstamped_envelope_reports_no_sequence_after_a_round_trip() {
    // `None` must survive as `None`. Reading it as zero would report a permanent gap at the bottom of
    // the range for every relay-sourced or client-sourced packet, which legitimately carry none.
    let bytes = current().to_datagram().expect("encodes");
    let decoded = QuicNetworkPacket::from_datagram(&bytes).expect("decodes");

    assert_eq!(decoded.sequence(), None);
}

#[test]
fn a_fully_stamped_envelope_costs_less_than_the_owner_it_replaced() {
    // Identity moved from the client's claim to the server's finding, and got cheaper doing it. The
    // old `client_id` was 32 random bytes plus a length prefix on every single datagram; the
    // replacement is a canonical name plus a varint device id, and it arrives alongside a sequence
    // number the old envelope had no room for at all.
    //
    // Measured against a 1150-byte cap, so the direction of this arithmetic is worth knowing rather
    // than discovering.
    let mut stamped = current();
    stamped.sender = Some(PacketSender::player(identity("Alaydriem"), 7));
    stamped.stamp(u32::MAX);

    let owned = LegacyEnvelope {
        packet_type: PacketType::HealthCheck,
        owner: Some(LegacyOwner {
            name: "Alaydriem".to_string(),
            client_id: vec![0u8; 32],
        }),
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
    };

    let new_size = stamped.to_datagram().expect("encodes").len();
    let old_size = postcard::to_stdvec(&owned).expect("legacy encodes").len();

    assert!(
        new_size < old_size,
        "3.0.0 envelope is {new_size} bytes against 2.1.0's {old_size}"
    );
    assert!(new_size < MAX_DATAGRAM_SIZE);
}

// An envelope with a realistic payload, so the offset assertions below are not made against a
// datagram short enough to hide an off-by-one.
fn audio_envelope() -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
            vec![0x5a; 160],
            None,
            Some(true),
        )),
        ..Default::default()
    }
}

#[test]
fn sequence_bytes_sit_at_a_fixed_offset() {
    // The fan-out serializes one envelope per frame and rewrites `SEQ_VALUE_RANGE` for each
    // recipient instead of re-encoding. That is only sound while `seq` is the first field and
    // fixed-width. A field reorder or a dropped `fixint` attribute would break it silently — the
    // datagram would still serialize, and the recipient would read a mangled packet.
    //
    // One envelope, re-stamped in place, because that is what the fan-out holds. A second
    // `audio_envelope()` would carry a second `AudioFramePacket::new` timestamp, so the two
    // encodes would differ in the timestamp rather than in the sequence whenever the
    // millisecond turns over between them.
    let mut packet = audio_envelope();
    packet.stamp(0);
    let template = packet.to_datagram().expect("template serializes");

    for value in [1u32, 2, 127, 128, 65_535, 16_777_216, u32::MAX] {
        let mut patched = template.clone();
        patched[QuicNetworkPacket::SEQ_VALUE_RANGE].copy_from_slice(&value.to_le_bytes());

        packet.stamp(value);
        let encoded = packet.to_datagram().expect("serializes");

        assert_eq!(
            patched, encoded,
            "patching the sequence range diverges from a fresh encode at seq={value}"
        );
    }
}

#[test]
fn the_sequence_occupies_a_constant_width_regardless_of_value() {
    let mut small = audio_envelope();
    small.stamp(1);
    let mut large = audio_envelope();
    large.stamp(u32::MAX);

    assert_eq!(
        small.to_datagram().unwrap().len(),
        large.to_datagram().unwrap().len(),
        "encoded length varies with the sequence value, so the field is still a varint"
    );
}

#[test]
fn an_absent_sequence_encodes_as_a_bare_tag() {
    // The `None` case carries no value bytes at all, so a patcher must never assume the range is
    // present. `ConnectionSequence` builds its template from a stamped envelope for this reason.
    let unstamped = audio_envelope().to_datagram().expect("serializes");
    let mut stamped_packet = audio_envelope();
    stamped_packet.stamp(0);
    let stamped = stamped_packet.to_datagram().expect("serializes");

    assert_eq!(unstamped[QuicNetworkPacket::SEQ_TAG_OFFSET], 0);
    assert_eq!(stamped[QuicNetworkPacket::SEQ_TAG_OFFSET], 1);
    assert_eq!(
        stamped.len() - unstamped.len(),
        QuicNetworkPacket::SEQ_VALUE_RANGE.len(),
        "a stamped envelope must cost exactly the fixed-width value"
    );
}

#[test]
fn a_client_built_packet_carries_no_identity() {
    // A packet a client builds carries no identity, because there is no field in which to declare
    // one. The absence is the security property: identity comes from the certificate the server
    // authenticated, never from the datagram. If a claimable field is ever re-added, this fails.
    let packet = QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
            vec![1, 2, 3],
            None,
            Some(true),
        )),
        ..Default::default()
    };

    assert!(packet.sender.is_none());
}

// The fan-out rewrites a fixed byte range on an already-encoded envelope. All four sender
// shapes must leave that range where it is, or one of them silently corrupts either the
// sequence or the sender on every recipient's copy.
#[test]
fn the_sequence_range_is_unmoved_by_every_sender_shape() {
    let shapes = [
        PacketSender::player(identity("Alaydriem"), 7),
        PacketSender::relayed(identity("FarAway")),
        PacketSender::Device(7),
        PacketSender::for_service(PacketSender::CHANNEL_API),
    ];

    for sender in shapes {
        let mut packet = audio_envelope();
        packet.sender = Some(sender.clone());
        packet.stamp(0x1234_5678);

        let bytes = packet.to_datagram().expect("encodes");
        let decoded = QuicNetworkPacket::from_datagram(&bytes).expect("decodes");

        assert_eq!(decoded.sequence(), Some(0x1234_5678), "sender {sender:?}");
        assert_eq!(decoded.sender, Some(sender.clone()), "sender {sender:?}");
        assert_eq!(
            &bytes[QuicNetworkPacket::SEQ_VALUE_RANGE],
            &0x1234_5678u32.to_le_bytes(),
            "sender {sender:?}"
        );
    }
}

// The fan-out patches a fixed byte range on an already-encoded envelope. A frame carrying a
// speaker is longer than one without, and if that moved the range, every recipient's copy would
// have either a corrupt sequence or a corrupt speaker.
#[test]
fn the_sequence_range_is_unmoved_by_an_attached_speaker() {
    let mut packet = audio_envelope();
    if let QuicNetworkPacketData::AudioFrame(ref mut audio) = packet.data {
        audio.speaker = Some(SpeakerPosition::new(
            Coordinate {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            false,
        ));
    }
    packet.stamp(0x1234_5678);

    let bytes = packet.to_datagram().expect("encodes");
    let decoded = QuicNetworkPacket::from_datagram(&bytes).expect("decodes");

    assert_eq!(decoded.sequence(), Some(0x1234_5678));
    assert_eq!(
        &bytes[QuicNetworkPacket::SEQ_VALUE_RANGE],
        &0x1234_5678u32.to_le_bytes()
    );
}
