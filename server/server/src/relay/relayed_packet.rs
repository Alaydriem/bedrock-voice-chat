use common::structs::packet::QuicNetworkPacket;

// Server-internal origin tag for a packet moving through the relay subsystem.
//
// Loop prevention is structural: this marker lives ONLY in memory and is never
// serialized onto the QUIC wire (the shared `QuicNetworkPacket` struct is left
// untouched). Packets that arrived from a peer link are wrapped `FromPeer` and
// are fed into local broadcast only — they are never handed to the outbound
// forwarder, enforcing single-hop relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketOrigin {
    Local,
    FromPeer,
}

// A `QuicNetworkPacket` paired with its server-internal origin. Used on the
// relay forwarding channels so the outbound fan-out can refuse to re-relay
// peer-origin traffic without inspecting (or mutating) the wire format.
#[derive(Debug, Clone)]
pub struct RelayedPacket {
    pub packet: QuicNetworkPacket,
    pub origin: PacketOrigin,
}

impl RelayedPacket {
    pub fn local(packet: QuicNetworkPacket) -> Self {
        Self {
            packet,
            origin: PacketOrigin::Local,
        }
    }

    pub fn from_peer(packet: QuicNetworkPacket) -> Self {
        Self {
            packet,
            origin: PacketOrigin::FromPeer,
        }
    }

    pub fn is_relayed(&self) -> bool {
        matches!(self.origin, PacketOrigin::FromPeer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::structs::packet::{AudioFramePacket, PacketType, QuicNetworkPacketData};

    fn sample_packet() -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: None,
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![1, 2, 3],
                48000,
                None,
                Some(true),
            )),
            // Not a server fan-out to one connection, so this envelope carries no sequence.
            ..Default::default()
        }
    }

    #[test]
    fn local_is_not_relayed() {
        let p = RelayedPacket::local(sample_packet());
        assert!(!p.is_relayed());
        assert_eq!(p.origin, PacketOrigin::Local);
    }

    #[test]
    fn from_peer_is_relayed() {
        let p = RelayedPacket::from_peer(sample_packet());
        assert!(p.is_relayed());
        assert_eq!(p.origin, PacketOrigin::FromPeer);
    }
}
