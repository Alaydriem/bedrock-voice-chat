pub mod acme;
pub mod age;
pub mod bedrock;
pub mod cors;
pub mod features;
pub mod meridian;
pub mod minecraft;
pub mod enrollment;
pub mod peer;
pub mod registry;
pub mod tls;

pub use acme::{Acme, AcmeProviderKind};
pub use age::Age;
pub use bedrock::BedrockConfig;
pub use bedrock::BedrockServerEntry;
pub use cors::Cors;
pub use features::Features;
pub use meridian::Meridian;
pub use minecraft::Minecraft;
pub use enrollment::Enrollment;
pub use peer::PeerConfig;
pub use registry::Registry;
pub use tls::Tls;

use serde::{Deserialize, Serialize};

// Dual-stack. A v6 wildcard bind serves IPv4 peers as well, because
// s2n-quic-platform clears IPV6_V6ONLY on the socket it creates rather than
// deferring to the host's sysctl. An operator on a host without IPv6 needs no
// config change: the QUIC listener falls back to the v4 wildcard.
fn default_listen() -> String {
    "::".to_string()
}

fn default_http_port() -> u32 {
    443
}

fn default_quic_port() -> u32 {
    443
}

fn default_advertised_quic_ports() -> Vec<u32> {
    vec![443, 28280]
}

fn default_assets_path() -> String {
    "./assets".to_string()
}

fn default_peer_port() -> Option<u16> {
    Some(28284)
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Server {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_http_port")]
    pub port: u32,
    #[serde(default = "default_quic_port")]
    pub quic_port: u32,
    #[serde(default = "default_advertised_quic_ports")]
    pub advertised_quic_ports: Vec<u32>,
    #[serde(default = "default_assets_path")]
    pub assets_path: String,
    #[serde(default)]
    pub tls: Tls,
    #[serde(default)]
    pub cors: Cors,
    #[serde(default)]
    pub minecraft: Minecraft,
    #[serde(default)]
    pub features: Features,
    #[serde(default)]
    pub meridian: Option<Meridian>,
    #[serde(default)]
    pub bedrock: BedrockConfig,
    #[serde(default)]
    pub age: Age,

    // Declared peers, keyed by the block label. hcl-rs deserializes a labeled
    // block into a label-keyed object, so `peer "name" { ... }` arrives as a map
    // rather than a list.
    #[serde(default)]
    pub peers: std::collections::HashMap<String, PeerConfig>,

    // Binds the peer socket on a server that has not declared a peer yet, which is every
    // server before its first pairing. Declared peers still enable it on their own, so a
    // deployment that predates this field keeps binding untouched.
    #[serde(default = "default_true")]
    pub peering: bool,

    // Enrolling with the BVC registry for a hostname and a certificate. Absent means
    // this server brings its own domain, which is the pre-existing arrangement.
    #[serde(default)]
    pub enrollment: Enrollment,

    // The UDP port the peer endpoint binds.
    //
    // Defaulted rather than left to the operating system, because this port is part
    // of the ticket a peer is given: an ephemeral one changes on every restart and
    // silently invalidates what the far side was already handed. A peer reached only
    // through the relay does not depend on it.
    //
    // `0` asks for an ephemeral port back, the same way `quic_port` reads it.
    #[serde(default = "default_peer_port")]
    pub peer_port: Option<u16>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            port: default_http_port(),
            quic_port: default_quic_port(),
            advertised_quic_ports: default_advertised_quic_ports(),
            assets_path: default_assets_path(),
            tls: Tls::default(),
            cors: Cors::default(),
            minecraft: Minecraft::default(),
            features: Features::default(),
            meridian: None,
            bedrock: BedrockConfig::default(),
            age: Age::default(),
            peers: std::collections::HashMap::new(),
            peering: true,
            enrollment: Enrollment::default(),
            peer_port: default_peer_port(),
        }
    }
}

impl Server {
    // Whether to bind the peer endpoint at all.
    //
    // Declared peers enable it on their own so an existing deployment needs no edit, and
    // the flag enables it for a server whose peers will arrive by pairing rather than by
    // configuration.
    pub fn peering_enabled(&self) -> bool {
        self.peering || !self.peers.is_empty()
    }

    // Used when a v6 bind fails, which means the host has no IPv6 stack.
    pub const FALLBACK_LISTEN: &'static str = "0.0.0.0";

    // Which source certificate material comes from. Exactly one, or none.
    //
    // Returns the sentence an operator reads, naming both sources they set, because
    // the failure is always that two are configured rather than that one is wrong.
    //
    // A blank token is no token: an operator who cleared the value rather than
    // deleting the line has configured nothing, and reading it as a source would
    // refuse a server that is otherwise correct.
    pub fn tls_source_conflict(&self) -> Option<String> {
        let manual = !self.tls.certificate.is_empty() || !self.tls.key.is_empty();
        let acme = self.tls.acme.is_some();
        let enrollment = self.enrollment.token().is_some();

        match (enrollment, manual, acme) {
            (true, true, _) => Some(
                "server.enrollment.token and tls.certificate/tls.key are mutually exclusive; \
                 remove one"
                    .to_string(),
            ),
            (true, _, true) => Some(
                "server.enrollment.token and tls.acme are mutually exclusive; remove one"
                    .to_string(),
            ),
            (_, true, true) => Some(
                "tls.acme and tls.certificate/tls.key are mutually exclusive; remove one"
                    .to_string(),
            ),
            _ => None,
        }
    }

    // A bare IPv6 address needs brackets before a port is appended. An address an
    // operator already bracketed parses as neither an IpAddr nor a host:port pair,
    // so it falls through unbracketed and is correct as-is.
    //
    // The QUIC listener can take a wildcard v6 address on every platform:
    // s2n-quic-platform clears IPV6_V6ONLY on the socket it creates, so IPv4 peers
    // still arrive (as v4-mapped) rather than being refused.
    pub fn quic_bind_addr(&self, port: u32) -> String {
        match self.listen.parse::<std::net::IpAddr>() {
            Ok(std::net::IpAddr::V6(_)) => format!("[{}]:{}", self.listen, port),
            _ => format!("{}:{}", self.listen, port),
        }
    }

    // Rocket's `address` figment key wants a bare IpAddr.
    //
    // The HTTP listener cannot follow QUIC onto a wildcard v6 address everywhere.
    // Rocket binds through tokio without touching IPV6_V6ONLY, which Windows
    // defaults to *enabled* — so `::` there yields an IPv6-only listener that
    // refuses every IPv4 client. Linux defaults the same flag to disabled
    // (`net.ipv6.bindv6only`), which is why one key served both listeners until now.
    //
    // Where the wildcard cannot be dual-stack, the IPv4 wildcard is the safer
    // reading of "listen everywhere": it keeps IPv4 clients working, at the cost of
    // IPv6 HTTP on that host. `http_listen_is_downgraded` reports when that applies
    // so startup can say so out loud.
    pub fn http_listen_ip(&self) -> &str {
        if self.http_listen_is_downgraded() {
            return Self::FALLBACK_LISTEN;
        }

        self.unbracketed_listen()
    }

    // True when this platform cannot give the HTTP listener a dual-stack wildcard
    // and the configured address is one.
    pub fn http_listen_is_downgraded(&self) -> bool {
        self.listen_is_wildcard_v6() && !Self::wildcard_v6_is_dual_stack()
    }

    pub fn listen_is_wildcard_v6(&self) -> bool {
        matches!(
            self.unbracketed_listen().parse::<std::net::IpAddr>(),
            Ok(std::net::IpAddr::V6(v6)) if v6.is_unspecified()
        )
    }

    // Whether a wildcard IPv6 TCP bind on this host accepts IPv4 peers. Rocket binds
    // through tokio without touching IPV6_V6ONLY, so the answer is entirely the
    // platform's.
    //
    // Windows defaults the flag to enabled, and Rocket 0.5 exposes no way to hand it
    // a pre-configured socket (`server.rs` binds internally; `http_server` is
    // `pub(crate)`), so there is nothing to clear it with.
    #[cfg(windows)]
    fn wildcard_v6_is_dual_stack() -> bool {
        false
    }

    // Linux defaults `net.ipv6.bindv6only` to 0, which is why one key served both
    // listeners. It is a sysctl, though, not a constant: a hardened host or a
    // container image can set it to 1, and a `::` bind is then IPv6-only here too.
    // Reading it is what keeps that from being a silent loss of every IPv4 client.
    #[cfg(target_os = "linux")]
    fn wildcard_v6_is_dual_stack() -> bool {
        match std::fs::read_to_string("/proc/sys/net/ipv6/bindv6only") {
            Ok(value) => value.trim() == "0",
            // Unreadable is not evidence of a problem — a container may not mount
            // /proc/sys — so the documented default stands rather than downgrading a
            // deployment that works.
            Err(_) => true,
        }
    }

    // macOS and the BSDs default their equivalent (`net.inet6.ip6.v6only`) to 0.
    #[cfg(not(any(windows, target_os = "linux")))]
    fn wildcard_v6_is_dual_stack() -> bool {
        true
    }

    pub fn unbracketed_listen(&self) -> &str {
        self.listen.trim_start_matches('[').trim_end_matches(']')
    }

    // Public UDP ports a client should try, in the operator's preferred order.
    // Deliberately independent of `quic_port`: the server binds one socket, but a
    // fronting proxy and a direct port publish can both deliver to it, so
    // reachability is many-to-one.
    //
    // The default names both public ways in, so a client whose network drops UDP/443
    // has a second port to try without the operator configuring anything. A list in
    // `config.hcl` replaces it outright — including the bind port, which is why an
    // operator who moves `quic_port` names the list as well.
    pub fn quic_ports(&self) -> Vec<u32> {
        if self.advertised_quic_ports.is_empty() {
            return vec![self.quic_port];
        }

        self.advertised_quic_ports.clone()
    }
}
