use std::net::SocketAddr;

/// What a preflight needs to know about the server it proves reachable.
#[derive(Debug, Clone)]
pub struct PlayPreflightRequest {
    // The configured public address, or `None` for the one the registry sees.
    pub address: Option<String>,
    // The public port players dial (server.enrollment.port(server.port)).
    pub port: u16,
    pub quic_port: u16,
    // Where the responder binds: the address the demultiplexer binds.
    pub listen: SocketAddr,
}
