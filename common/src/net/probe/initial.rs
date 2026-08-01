// A QUIC Initial whose version is drawn from the RFC 9000 section 6.3 reserved
// pattern, which no endpoint may support. A conforming server answers it with a
// Version Negotiation packet, so it proves a QUIC listener is present without any
// certificate on either side.
pub struct ProbeInitialPacket {
    datagram: Vec<u8>,
    source_cid: [u8; Self::CID_LEN],
}

impl ProbeInitialPacket {
    pub const CID_LEN: usize = 8;

    // RFC 9000 section 14.1: a server drops an unsupported-version Initial below
    // this size rather than answering it.
    pub const DATAGRAM_LEN: usize = 1200;

    const VERSION: u32 = 0x0a0a_0a0a;

    // Long header, fixed bit, Initial type, and a four-byte packet number.
    const FIRST_BYTE: u8 = 0xc3;

    pub fn new() -> Self {
        let mut dcid = [0u8; Self::CID_LEN];
        let mut scid = [0u8; Self::CID_LEN];

        for byte in dcid.iter_mut() {
            *byte = rand::random::<u8>();
        }
        for byte in scid.iter_mut() {
            *byte = rand::random::<u8>();
        }

        Self::with_cids(dcid, scid)
    }

    pub fn with_cids(dcid: [u8; Self::CID_LEN], scid: [u8; Self::CID_LEN]) -> Self {
        let mut datagram = Vec::with_capacity(Self::DATAGRAM_LEN);

        datagram.push(Self::FIRST_BYTE);
        datagram.extend_from_slice(&Self::VERSION.to_be_bytes());
        datagram.push(Self::CID_LEN as u8);
        datagram.extend_from_slice(&dcid);
        datagram.push(Self::CID_LEN as u8);
        datagram.extend_from_slice(&scid);
        datagram.push(0x00);

        // The Length field covers the packet number and everything after it. All
        // of that is padding here, so the value is known before it is written, and
        // a two-byte varint holds it for any datagram this size.
        let header_len = datagram.len() + 2;
        let remainder = Self::DATAGRAM_LEN - header_len;
        let varint = 0x4000u16 | (remainder as u16);
        datagram.extend_from_slice(&varint.to_be_bytes());

        datagram.extend_from_slice(&0u32.to_be_bytes());
        datagram.resize(Self::DATAGRAM_LEN, 0x00);

        Self {
            datagram,
            source_cid: scid,
        }
    }

    pub fn datagram(&self) -> &[u8] {
        &self.datagram
    }

    pub fn source_cid(&self) -> &[u8] {
        &self.source_cid
    }

    // A Version Negotiation packet is a long header whose version field is zero
    // and whose Destination Connection ID echoes the Source Connection ID we
    // sent. The echo is what separates our reply from a stray packet.
    pub fn accepts_reply(&self, reply: &[u8]) -> bool {
        if reply.len() < 6 {
            return false;
        }

        if reply[0] & 0x80 == 0 {
            return false;
        }

        if u32::from_be_bytes([reply[1], reply[2], reply[3], reply[4]]) != 0 {
            return false;
        }

        let dcid_len = reply[5] as usize;
        if reply.len() < 6 + dcid_len {
            return false;
        }

        reply[6..6 + dcid_len] == self.source_cid
    }
}

impl Default for ProbeInitialPacket {
    fn default() -> Self {
        Self::new()
    }
}
