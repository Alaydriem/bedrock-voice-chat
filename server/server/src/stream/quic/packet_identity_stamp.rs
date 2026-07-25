use common::structs::packet::QuicNetworkPacket;

// Rewrites an inbound packet's declared owner name to the connection's
// mTLS-authenticated identity.
//
// The wire `owner.name` is client-controlled, so it is a claim, not a fact. Applying
// this at the input boundary makes every downstream consumer — `get_author()`, the
// cache guards, membership keying, outbound broadcast — read an authenticated value
// without each having to know about certificates.
//
// `client_id` is deliberately untouched: it is the client's per-device routing key,
// and preserving it is what keeps one player's two devices separately addressable.
// An owner-less packet is left alone rather than given a synthesized owner, because
// there is no client_id to attribute it to.
pub struct PacketIdentityStamp;

impl PacketIdentityStamp {
    pub fn apply(packet: &mut QuicNetworkPacket, authenticated_name: &str) {
        if let Some(owner) = packet.owner.as_mut() {
            if owner.name != authenticated_name {
                tracing::debug!(
                    claimed = %owner.name,
                    authenticated = %authenticated_name,
                    "Rewriting packet owner to the authenticated identity"
                );
                owner.name = authenticated_name.to_string();
            }
        }
    }
}
