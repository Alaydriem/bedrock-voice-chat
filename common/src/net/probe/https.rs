use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Instant;

use crate::net::NetTimeouts;
use crate::rustls::BvcClientTls;
use crate::structs::reachability::{AddressFamily, AnsweredVia, ReachabilityOutcome};

pub struct HttpsProbe;

impl HttpsProbe {
    // Pinning the local address is what forces the request onto the family being
    // measured; the connector would otherwise pick for itself and the result would
    // not describe either family.
    pub async fn probe(url: &str, family: AddressFamily) -> ReachabilityOutcome {
        let local: IpAddr = match family {
            AddressFamily::Ipv4 => Ipv4Addr::UNSPECIFIED.into(),
            AddressFamily::Ipv6 => Ipv6Addr::UNSPECIFIED.into(),
        };

        // The play roots are what let this probe validate a play-zone server at all;
        // every other name is validated against the public roots, as before.
        let builder = match BvcClientTls::http_client_builder(None, None) {
            Ok(builder) => builder,
            Err(e) => {
                #[cfg(all(debug_assertions, feature = "dev-trust"))]
                let hint = "check BVC_DEV_TRUST_ROOTS and BVC_DEV_TRUST_ROOTS_PEM";
                #[cfg(not(all(debug_assertions, feature = "dev-trust")))]
                let hint = "";
                curia::warn!(
                    "HTTPS probe TLS client could not be built",
                    { "url": url.to_string(), "error": e.to_string(), "hint": hint }
                );
                return ReachabilityOutcome::Silent;
            }
        };

        let client = match builder
            .timeout(NetTimeouts::HTTPS)
            .local_address(local)
            .build()
        {
            Ok(client) => client,
            Err(_) => return ReachabilityOutcome::Silent,
        };

        let started = Instant::now();
        let status = client
            .get(url)
            .send()
            .await
            .ok()
            .map(|response| response.status().as_u16());
        let rtt_micros = started.elapsed().as_micros().min(u32::MAX as u128) as u32;

        Self::outcome_for(status, rtt_micros)
    }

    // Certificate validation stays on: an HTTPS endpoint whose certificate does
    // not verify is a problem an operator needs to see, not a detail to wave
    // through in the name of reporting it reachable.
    pub fn outcome_for(status: Option<u16>, rtt_micros: u32) -> ReachabilityOutcome {
        match status {
            Some(_) => ReachabilityOutcome::Answered {
                via: AnsweredVia::Https,
                rtt_micros,
            },
            None => ReachabilityOutcome::Silent,
        }
    }
}
