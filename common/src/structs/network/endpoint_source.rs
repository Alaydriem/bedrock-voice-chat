/// Where a server endpoint's port came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointSource {
    // The user typed the port.
    Explicit,
    // An SRV record named the port.
    Srv,
    // No port was typed and no record exists, so the default applies.
    Default,
}
