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

    // The address this server answers HTTPS on, published as the assigned name's A
    // record.
    //
    // Declared rather than discovered: an operator behind NAT, behind CGNAT, or on a
    // LAN has no address the registry could observe that would be right, and one guessed
    // from a connection's source is correct for a port-forwarded deployment and
    // silently wrong for every other.
    //
    // Optional in the schema, required in practice. Unset publishes no record, and the
    // assigned name then resolves to nothing — a client dials the hostname it was given
    // and fails in DNS, with a valid certificate sitting behind a name nobody can
    // reach. Startup says so out loud, because there is no other symptom that leads
    // back here.
    //
    // A private address is published but never verified — it resolves only on the
    // network that declared it. A public one is checked daily, because an address an
    // operator does not control would otherwise have the registry's zone fronting
    // somebody else's host.
    #[serde(default)]
    pub address: Option<String>,
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

    pub fn address(&self) -> Option<&str> {
        self.address
            .as_deref()
            .map(str::trim)
            .filter(|address| !address.is_empty())
    }
}
