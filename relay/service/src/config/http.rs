use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::acme::AcmeConfig;

fn default_port() -> u16 {
    443
}

fn default_bind() -> String {
    "::".to_string()
}

// The operator-facing surface: Discord's redirect target and the API the enrollment
// page calls.
//
// TLS is not optional and there is no field here that could disable it. A registry
// serving this unencrypted would be handing enrollment tokens to the network.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HttpConfig {
    // The name this registry is reached by, and the name its certificate is issued
    // for. Discord's registered redirect URI is derived from it.
    pub hostname: String,
    // The single origin allowed to redeem a claim. Everything else on this surface is
    // browser navigation and carries no CORS headers at all.
    pub page_origin: String,
    #[serde(default = "default_port")]
    pub port: u16,
    // The address the HTTPS listener binds.
    //
    // `::` is a dual-stack wildcard rather than an IPv6-only one: the listener clears
    // `IPV6_V6ONLY` explicitly, so IPv4 clients are served on every platform instead of
    // only on the ones whose default happens to allow it.
    //
    // Set `0.0.0.0` for IPv4 only, or a specific address to bind one interface.
    #[serde(default = "default_bind")]
    pub bind: String,
    // Labelled by provider, so a second becomes another label rather than a breaking
    // change to this one. hcl-rs deserializes `acme "cloudflare" { .. }` into a
    // label-keyed map, the same shape the server's `peers "name"` blocks use.
    #[serde(default)]
    pub acme: HashMap<String, AcmeConfig>,
}

impl HttpConfig {
    const CLOUDFLARE: &'static str = "cloudflare";

    // Parsed rather than taken as text, so a typo stops the start with a message that
    // names the value instead of failing later inside the socket call.
    pub fn bind_address(&self) -> Result<std::net::IpAddr, String> {
        self.bind
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse()
            .map_err(|_| format!("http.bind is not an IP address: {}", self.bind))
    }

    // Built from the hostname rather than configured separately. The value must match
    // what is registered with Discord byte for byte, and two fields that must agree
    // are two fields that eventually do not.
    pub fn redirect_uri(&self) -> String {
        format!("https://{}/oauth/callback", self.hostname)
    }

    pub fn cloudflare(&self) -> Option<&AcmeConfig> {
        self.acme.get(Self::CLOUDFLARE)
    }
}
