use common::structs::packet::QuicNetworkPacket;
use common::structs::relay::wire::datagram::GroupScope;

// Where a peer's admitted packets go.
//
// A trait so the plane can be tested without the server's broadcast loop, and so
// the plane never learns how local delivery works.
//
// `group` is the linked group the frame claims. It travels beside the packet, never in it, so
// it cannot reach a client-bound packet.
pub trait PeerSink: Send + Sync {
    fn publish(&self, packet: QuicNetworkPacket, group: Option<GroupScope>);
}
