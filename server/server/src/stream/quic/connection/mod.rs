mod at_capacity;
mod capacity;
mod entry;
mod id_format;
mod registry;
mod route_rejection_counts;
mod routed_packet;
// Public so the integration crate can drive the one invariant this mechanism rests on: a sequence
// number is consumed only when a datagram is actually produced for a connection.
pub mod sequence;

pub use at_capacity::AtCapacity;
pub use capacity::CapacityPolicy;
pub(crate) use entry::ConnectionEntry;
pub use id_format::PrefixedConnectionIdFormat;
pub use registry::ConnectionRegistry;
pub use route_rejection_counts::RouteRejectionCounts;
pub use routed_packet::RoutedPacket;
pub use sequence::ConnectionSequence;
