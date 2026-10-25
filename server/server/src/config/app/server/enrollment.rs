use serde::{Deserialize, Serialize};

// Enrolling with the BVC registry to be given a hostname and a certificate.
//
// One block rather than fields scattered across `server`, because they are only ever
// set together and are meaningless apart: an address without a token has no name to
// attach to it.
//
// Where the registry lives is NOT here. Address observation dials it too, and that is
// for every server rather than only entitled members, so it belongs to neither
// feature's block.
#[derive(Serialize, Deserialize, Debug, Clone, Default, schemars::JsonSchema)]
pub struct Enrollment {
    // The single-use value an operator pastes exactly once.
    //
    // Spent on first boot. Afterwards the node key alone authenticates every
    // exchange, so a configuration file leaked after enrollment grants nothing — and
    // the token can be left in place or removed, whichever the operator prefers.
    #[serde(default)]
    pub token: Option<String>,

    // The address this server answers HTTPS on, sent at each relay check-in and published
    // as the assigned name's A or AAAA record. Unset publishes no record. A public
    // address is probed for the check-in nonce; a private one is not.
    #[serde(default)]
    pub address: Option<String>,

    // The port the relay probes for the enrollment nonce. Defaults to `server.port`; set
    // it when a proxy or port forward answers on a different public port.
    #[serde(default)]
    pub port: Option<u16>,
}

impl Enrollment {
    // Whether this server is configured to enroll at all. A blank token is no token:
    // an operator who cleared the value rather than deleting the line has configured
    // nothing.
    pub fn token(&self) -> Option<&str> {
        self.token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
    }

    // `None` when no configured port is set and `server_port` does not fit a port.
    pub fn port(&self, server_port: u32) -> Option<u16> {
        self.port.or_else(|| u16::try_from(server_port).ok())
    }

    pub fn address(&self) -> Option<&str> {
        self.address
            .as_deref()
            .map(str::trim)
            .filter(|address| !address.is_empty())
    }
}
