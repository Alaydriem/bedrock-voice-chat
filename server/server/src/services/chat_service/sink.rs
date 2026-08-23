use common::structs::packet::{ChatMessagePacket, ChatRejectedPacket};

/// Where a line goes once the service has accepted it.
///
/// QUIC is the only implementation today. A Discord bridge would register as a second sink
/// rather than reaching into the service, which is the whole reason this is a trait.
pub trait ChatSink: Send + Sync {
    /// `author_identity` is set only for a line the app sent, and only once per send, so an
    /// implementation can guarantee the sender receives their own copy. Delivery is otherwise
    /// addressed by where players are standing, and a sender who is not in game is not there.
    fn deliver(&self, world_uuid: &str, author_identity: Option<&str>, packet: &ChatMessagePacket);

    /// Tells one sender their line was refused. Never world-scoped: nobody else saw it.
    fn deliver_rejection(&self, identity: &str, packet: &ChatRejectedPacket);
}
