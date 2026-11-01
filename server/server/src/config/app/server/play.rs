use serde::{Deserialize, Serialize};

// A play-zone name and a 15 day certificate from the BVC registry.
//
// Present, even empty, to turn it on: `play {}` is a complete block. The registry assigns
// the name and signs the certificate for it, so the only thing an operator may state is
// where players reach the server.
#[derive(Serialize, Deserialize, Debug, Clone, Default, schemars::JsonSchema)]
pub struct PlayTls {
    // The address this server answers on, sent at each relay check-in and published as the
    // play name's A or AAAA record. Unset leaves the registry to use the address it observes
    // the check-in coming from. The registry refuses a private address.
    #[serde(default)]
    pub address: Option<String>,
}

impl PlayTls {
    // A blank address is no address: an operator who cleared the value rather than deleting
    // the line has configured nothing, and a blank one would ask the registry to publish an
    // empty record.
    pub fn address(&self) -> Option<&str> {
        self.address
            .as_deref()
            .map(str::trim)
            .filter(|address| !address.is_empty())
    }

    // The registry accepts an IP literal and nothing else, and refuses a hostname only at
    // the hourly check-in, as a warning nobody reads. Said once, at startup.
    pub fn problem(&self) -> Option<String> {
        let address = self.address()?;
        match address.parse::<std::net::IpAddr>() {
            Ok(_) => None,
            Err(_) => Some(format!(
                "server.tls.play.address = \"{address}\" is not an IP address; the registry \
                 publishes an address, not a hostname"
            )),
        }
    }
}
