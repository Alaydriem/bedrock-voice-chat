/// Where a target actually is.
///
/// Kept off the wire on purpose: a controller picks a name, and handing it a host and port
/// would make it responsible for an address it has no business knowing.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedAddress {
    Proxy {
        host: String,
        port: u16,
        protocol_version: Option<u32>,
    },
    Realm {
        realm_id: u64,
    },
}
