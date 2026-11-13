/// What a DNS responder fixture does with a query.
#[derive(Clone, Debug)]
pub enum DnsScript {
    // An SRV answer naming the queried host.
    Answer { port: u16, ttl: u32 },
    // NXDOMAIN with no records.
    NxDomain,
}
