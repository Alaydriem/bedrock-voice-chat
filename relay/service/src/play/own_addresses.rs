use std::net::{IpAddr, SocketAddr};

use common::curia;

// The addresses that reach this host: its interfaces and the address it advertises.
pub struct OwnAddresses;

impl OwnAddresses {
    pub fn collect(advertised: Option<SocketAddr>) -> Vec<IpAddr> {
        let mut addresses = match if_addrs::get_if_addrs() {
            Ok(interfaces) => interfaces
                .iter()
                .map(|interface| interface.ip().to_canonical())
                .collect(),
            Err(e) => {
                curia::warn!("The registry's interface addresses could not be listed. Preflight cannot refuse its own address.", { "error": e.to_string() });
                Vec::new()
            }
        };
        if let Some(advertised) = advertised {
            addresses.push(advertised.ip().to_canonical());
        }

        addresses
    }
}
