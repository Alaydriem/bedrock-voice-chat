use bvc_server_lib::stream::quic::connection_sequence::ConnectionSequence;
use common::structs::packet::{
    HealthCheckPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

fn envelope() -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::HealthCheck,
        owner: None,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        seq: None,
    }
}

fn sequence_of(bytes: &[u8]) -> Option<u32> {
    QuicNetworkPacket::from_datagram(bytes)
        .expect("round-trips")
        .sequence()
}

#[test]
fn an_unstamped_envelope_reports_no_sequence() {
    // A peer predating this field must read as unmeasured, never as sequence zero: a receiver that
    // treated absence as zero would report a permanent gap at the bottom of the range.
    assert_eq!(envelope().sequence(), None);
}

#[test]
fn stamping_is_monotonic_and_gapless() {
    let sequence = ConnectionSequence::new_shared();
    let mut packet = envelope();

    let observed: Vec<u32> = (0..5)
        .map(|_| {
            let bytes = sequence.stamp(&mut packet).expect("serializes");
            sequence_of(&bytes).expect("carries a sequence")
        })
        .collect();

    // Contiguous and ascending. Any gap here would be read as loss by every client.
    assert_eq!(observed, vec![0, 1, 2, 3, 4]);
}

#[test]
fn each_connection_numbers_independently() {
    // The sequence is per connection, not per server. A shared counter would appear to every client
    // as though most of its packets had been lost, since it would see only its own share.
    let first = ConnectionSequence::new_shared();
    let second = ConnectionSequence::new_shared();
    let mut packet = envelope();

    let a1 = sequence_of(&first.stamp(&mut packet).unwrap()).unwrap();
    let b1 = sequence_of(&second.stamp(&mut packet).unwrap()).unwrap();
    let a2 = sequence_of(&first.stamp(&mut packet).unwrap()).unwrap();

    assert_eq!(a1, 0);
    assert_eq!(b1, 0);
    assert_eq!(a2, 1);
}

#[test]
fn a_packet_that_is_never_stamped_consumes_no_sequence_number() {
    // The invariant the whole mechanism rests on. The router drops recipients for proximity,
    // channel membership and deafen distance before stamping; if a number were consumed by a packet
    // that is never sent, the client would see a gap that was never loss and report phantom loss.
    let sequence = ConnectionSequence::new_shared();
    let mut packet = envelope();

    let first = sequence_of(&sequence.stamp(&mut packet).unwrap()).unwrap();

    // Stand in for the router deciding not to send: build an envelope and simply never stamp it.
    let _suppressed = envelope();

    let second = sequence_of(&sequence.stamp(&mut packet).unwrap()).unwrap();

    assert_eq!(
        second,
        first + 1,
        "an unsent packet must not advance the sequence"
    );
}

#[test]
fn a_stamped_envelope_survives_the_datagram_round_trip() {
    // The field is last in the struct because postcard's format is positional. If it were moved,
    // this is what would fail.
    let sequence = ConnectionSequence::new_shared();
    let mut packet = envelope();
    for _ in 0..300 {
        let bytes = sequence.stamp(&mut packet).expect("serializes");
        assert!(sequence_of(&bytes).is_some());
    }

    // Past the single-byte varint boundary, so a multi-byte sequence is exercised too.
    let bytes = sequence.stamp(&mut packet).unwrap();
    assert_eq!(sequence_of(&bytes), Some(300));
}
