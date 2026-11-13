use std::net::IpAddr;

// A preflight the registry agreed to run: where to dial and what the answer must say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightTicket {
    pub address: IpAddr,
    pub port: u16,
    pub nonce: String,
    // Where the request came from, when that is known.
    pub observed: Option<IpAddr>,
}
