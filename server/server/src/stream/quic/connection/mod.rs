mod entry;
mod id_format;
mod registry;
mod routed_packet;
// Public so the integration crate can drive the one invariant this mechanism rests on: a sequence
// number is consumed only when a datagram is actually produced for a connection.
pub mod sequence;

pub(crate) use entry::ConnectionEntry;
pub use id_format::PrefixedConnectionIdFormat;
pub use registry::ConnectionRegistry;
pub use routed_packet::RoutedPacket;
pub use sequence::ConnectionSequence;
