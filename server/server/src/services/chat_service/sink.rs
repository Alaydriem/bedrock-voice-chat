use common::structs::packet::ChatMessagePacket;

/// Where a line goes once the service has accepted it.
///
/// QUIC is the only implementation today. A Discord bridge would register as a second sink
/// rather than reaching into the service, which is the whole reason this is a trait.
pub trait ChatSink: Send + Sync {
    fn deliver(&self, world_uuid: &str, packet: &ChatMessagePacket);
}
