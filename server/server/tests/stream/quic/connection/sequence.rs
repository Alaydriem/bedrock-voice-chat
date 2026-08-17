use bvc_server_lib::stream::quic::connection::ConnectionSequence;
use common::structs::packet::{
    HealthCheckPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

fn envelope() -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::HealthCheck,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        ..Default::default()
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
    let sequence = ConnectionSequence::new_shared();
    let mut packet = envelope();
    for _ in 0..300 {
        let bytes = sequence.stamp(&mut packet).expect("serializes");
        assert!(sequence_of(&bytes).is_some());
    }

    let bytes = sequence.stamp(&mut packet).unwrap();
    assert_eq!(sequence_of(&bytes), Some(300));
}

#[test]
fn patching_a_template_agrees_with_stamping_the_packet() {
    // Fan-out encodes one envelope per frame and patches its sequence bytes per recipient. If
    // `patch` ever disagreed with `stamp`, every listener would receive a subtly wrong datagram
    // while the server logged nothing, so the two are held equal here.
    let stamped_sequence = ConnectionSequence::new_shared();
    let patched_sequence = ConnectionSequence::new_shared();

    let mut template_packet = envelope();
    template_packet.stamp(0);
    let template = template_packet.to_datagram().expect("template serializes");

    let mut packet = envelope();
    for _ in 0..300 {
        let stamped = stamped_sequence.stamp(&mut packet).expect("serializes");
        let patched = patched_sequence.patch(&template).expect("patches");
        assert_eq!(stamped, patched);
    }
}

#[test]
fn patching_advances_the_same_counter_stamping_does() {
    // One connection reaches both paths -- broadcasts patch, single-recipient control messages
    // stamp -- and a client reads one sequence stream. Two counters would look like loss.
    let sequence = ConnectionSequence::new_shared();
    let mut template_packet = envelope();
    template_packet.stamp(0);
    let template = template_packet.to_datagram().expect("template serializes");
    let mut packet = envelope();

    let first = sequence_of(&sequence.patch(&template).unwrap()).unwrap();
    let second = sequence_of(&sequence.stamp(&mut packet).unwrap()).unwrap();
    let third = sequence_of(&sequence.patch(&template).unwrap()).unwrap();

    assert_eq!((first, second, third), (0, 1, 2));
}

#[test]
fn a_template_too_short_to_hold_a_sequence_is_refused() {
    // An envelope serialized without a sequence has no value bytes to overwrite -- an absent
    // `Option` is a bare tag. Patching one would silently corrupt whatever followed, so a template
    // that cannot hold the range is rejected rather than written into.
    let sequence = ConnectionSequence::new_shared();

    assert!(sequence.patch(&[1u8, 0, 0]).is_none());
}
