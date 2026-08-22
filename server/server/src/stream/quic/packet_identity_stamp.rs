use common::structs::packet::{PacketSender, QuicNetworkPacket};

// Stamps an inbound packet with the connection's authenticated identity before anything
// downstream reads it.
//
// There is nothing to compare against and nothing to reject: a client has no field in which to
// claim an identity, so this writes rather than corrects. Applying it at the input boundary is
// what lets the cache guards, membership keying and outbound broadcast all read an
// authenticated value without any of them knowing about certificates.
//
// The write is unconditional. A packet arriving with a sender already set is either a relayed
// peer's or a forgery, and either way this server fans it out under its own authority.
pub struct PacketIdentityStamp;

impl PacketIdentityStamp {
    /// Stamps the connection's identity, which the caller holds as the text form the
    /// certificate carried. A value that is not canonical belongs to no player, so the
    /// packet is left unstamped and every guard downstream refuses to attribute it.
    pub fn apply(packet: &mut QuicNetworkPacket, identity: &str, device: u64) {
        match identity.parse::<common::PlayerIdentity>() {
            Ok(identity) => packet.sender = Some(PacketSender::player(identity, device)),
            Err(e) => tracing::warn!("Refusing to stamp a non-canonical identity: {e}"),
        }
    }
}
