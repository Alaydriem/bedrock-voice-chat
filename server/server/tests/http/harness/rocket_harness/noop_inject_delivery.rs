use bvc_server_lib::relay::LocalInjectDelivery;
use common::structs::packet::PeerPresenceInjectPacket;

// No-op realm-inject delivery so the `/api/relay/offer` route can mount in the
// harness (production injects the minted code through a live client). The offer
// route's mint + rate-limit behavior is what the tests exercise.
pub(super) struct NoopInjectDelivery;

impl LocalInjectDelivery for NoopInjectDelivery {
    fn deliver_inject(&self, _hashed_world: &str, _packet: PeerPresenceInjectPacket) {}
    fn deliver_announce(&self, _packet: common::structs::packet::PeerAnnounceInjectPacket) {}
}
