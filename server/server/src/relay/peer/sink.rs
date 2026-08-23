use common::structs::packet::QuicNetworkPacket;

// Where a peer's admitted packets go.
//
// A trait so the plane can be tested without the server's broadcast loop, and so
// the plane never learns how local delivery works.
pub trait PeerSink: Send + Sync {
    fn publish(&self, packet: QuicNetworkPacket);
}
