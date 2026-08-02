use common::structs::packet::{
    HealthCheckPacket, MAX_DATAGRAM_SIZE, PacketOwner, PacketType, QuicNetworkPacket,
    QuicNetworkPacketData,
};
use serde::{Deserialize, Serialize};

// Version skew for the envelope sequence field, established by measurement rather than assumption.
//
// The headline result: adding this field is a BREAKING protocol change. Postcard is not
// self-describing and its format is positional, so `#[serde(default)]` does nothing for a missing
// tail field — the decoder unconditionally reads the `Option` discriminant and runs off the end of
// the buffer. A tail addition is additive under a self-describing format; it is not under this one.
//
// `reference_versioned_codec_zero_packet` in this repo is the standing reminder that codec-level
// assumptions get verified. These tests are that verification, and they contradicted the assumption.

// The envelope as a peer without the field defines it.
#[derive(Serialize, Deserialize)]
struct LegacyEnvelope {
    packet_type: PacketType,
    owner: Option<PacketOwner>,
    data: QuicNetworkPacketData,
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
        owner: None,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        ..Default::default()
    }
}

#[test]
fn a_new_client_cannot_decode_an_old_servers_datagram() {
    // The breaking direction. An old server emits no sequence field, and the new decoder expects one
    // byte that is not there.
    //
    // This is why the field requires a protocol version bump rather than riding as an addition. It is
    // asserted rather than merely noted so that a future attempt to make the field "optional" fails
    // here instead of in the field.
    let bytes = postcard::to_stdvec(&legacy()).expect("legacy encodes");

    assert!(
        QuicNetworkPacket::from_datagram(&bytes).is_err(),
        "a new client decoding old bytes must fail loudly at the codec, not silently mis-parse"
    );
}

#[test]
fn an_old_client_tolerates_a_new_servers_datagram() {
    // The other direction happens to survive: postcard does not reject trailing bytes, so an old
    // decoder reads the three fields it knows and ignores the sequence discriminant.
    //
    // Asymmetric compatibility is worse than none, because it hides the break during a rollout — an
    // old client keeps working against a new server right up until it is itself updated. Recorded so
    // nobody reads that as evidence the change is safe.
    let bytes = current().to_datagram().expect("encodes");

    assert!(
        postcard::from_bytes::<LegacyEnvelope>(&bytes).is_ok(),
        "an old client is expected to ignore the trailing field"
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
fn the_field_costs_one_byte_when_unstamped() {
    // A discriminant byte on every packet, including those carrying no sequence. Against a 1150-byte
    // cap that is worth knowing rather than discovering.
    let with_field = current().to_datagram().expect("encodes").len();
    let without_field = postcard::to_stdvec(&legacy()).expect("legacy encodes").len();

    assert_eq!(with_field, without_field + 1);
    assert!(with_field < MAX_DATAGRAM_SIZE);
}
