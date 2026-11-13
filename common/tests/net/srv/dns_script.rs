/// What a DNS responder fixture does with a query.
#[derive(Clone, Debug)]
pub enum DnsScript {
    // An SRV answer. A `None` target names the queried host.
    Answer {
        port: u16,
        ttl: u32,
        target: Option<String>,
    },
    // NXDOMAIN with no records.
    NxDomain,
    // NOERROR with no records.
    Empty,
    // No reply at all.
    Silent,
    // For the DoH responder: accept the request and never answer. As a DNS script, silence.
    Hang,
    // Over UDP the TC bit and no records; over TCP the full answer.
    TruncatedUdp { port: u16, ttl: u32 },
}
