use common::net::ProbeInitialPacket;

// A server drops an unsupported-version Initial smaller than the QUIC minimum
// datagram instead of answering it, so the padding is what makes the probe work
// at all.
#[test]
fn the_probe_datagram_meets_the_quic_minimum() {
    let packet = ProbeInitialPacket::new();

    assert_eq!(packet.datagram().len(), 1200);
    assert_eq!(ProbeInitialPacket::DATAGRAM_LEN, 1200);
}

#[test]
fn the_probe_is_a_long_header_initial_with_the_fixed_bit_set() {
    let packet = ProbeInitialPacket::new();
    let first = packet.datagram()[0];

    assert_eq!(first & 0x80, 0x80, "header form bit must mark a long header");
    assert_eq!(first & 0x40, 0x40, "fixed bit must be set");
    assert_eq!(first & 0x30, 0x00, "long packet type must be Initial");
}

// RFC 9000 section 6.3 reserves the 0x?a?a?a?a pattern for versions that must
// never be supported, which is what guarantees a Version Negotiation reply rather
// than a real handshake.
#[test]
fn the_probe_version_is_reserved_for_forcing_negotiation() {
    let packet = ProbeInitialPacket::new();
    let datagram = packet.datagram();
    let version = u32::from_be_bytes([datagram[1], datagram[2], datagram[3], datagram[4]]);

    assert_eq!(version & 0x0f0f_0f0f, 0x0a0a_0a0a);
}

#[test]
fn the_declared_length_matches_the_bytes_that_follow_it() {
    let packet = ProbeInitialPacket::new();
    let datagram = packet.datagram();

    // 1 first byte + 4 version + 1 dcid len + 8 dcid + 1 scid len + 8 scid
    // + 1 zero-length token = 24 bytes before the Length varint.
    let length_offset = 24;
    assert_eq!(datagram[5], 8, "dcid length");
    assert_eq!(datagram[14], 8, "scid length");
    assert_eq!(datagram[23], 0, "token length varint must be a single zero byte");

    let varint = u16::from_be_bytes([datagram[length_offset], datagram[length_offset + 1]]);
    assert_eq!(varint & 0xc000, 0x4000, "must be a two-byte varint");

    let declared = (varint & 0x3fff) as usize;
    let actual = datagram.len() - (length_offset + 2);
    assert_eq!(declared, actual);
}

#[test]
fn two_probes_do_not_share_connection_ids() {
    let first = ProbeInitialPacket::new();
    let second = ProbeInitialPacket::new();

    assert_ne!(first.source_cid(), second.source_cid());
}

// Accepting any long-header reply would let a stray or spoofed packet report a
// dead server as alive. Echoing our Source Connection ID is what makes a reply
// ours.
#[test]
fn a_reply_is_accepted_only_when_it_echoes_our_source_connection_id() {
    let packet = ProbeInitialPacket::with_cids([1u8; 8], [2u8; 8]);

    let mut good = vec![0x80u8, 0, 0, 0, 0, 8];
    good.extend_from_slice(&[2u8; 8]);
    good.push(8);
    good.extend_from_slice(&[1u8; 8]);

    assert!(packet.accepts_reply(&good));

    let mut wrong_cid = good.clone();
    wrong_cid[6] = 0x99;
    assert!(!packet.accepts_reply(&wrong_cid));

    let mut wrong_version = good.clone();
    wrong_version[1] = 0x01;
    assert!(!packet.accepts_reply(&wrong_version));

    let mut short_header = good.clone();
    short_header[0] = 0x40;
    assert!(!packet.accepts_reply(&short_header));

    assert!(!packet.accepts_reply(&[]));
    assert!(!packet.accepts_reply(&good[..6]));
}
