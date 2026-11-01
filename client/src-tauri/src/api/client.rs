use common::reqwest::{Client as ReqwestClient, ClientBuilder};
use common::rustls::{BvcClientTls, PlayRoots, VerifierMode};
use common::structs::reachability::AddressFamilyPreference;
use rustls::pki_types::CertificateDer;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Client {
    ca_cert: String,
    pem: String,
    // The roots a play-zone name is checked against. `None` is the roots built into this
    // client; only a test supplies others.
    #[serde(skip)]
    play_roots: Option<Vec<CertificateDer<'static>>>,
    // `None` follows the build: permissive in a debug build, strict in a release build.
    #[serde(skip)]
    mode: Option<VerifierMode>,
}

impl Client {
    pub fn new(ca_cert: String, pem: String) -> Self {
        Self {
            ca_cert,
            pem,
            play_roots: None,
            mode: None,
        }
    }

    /// A client that checks play-zone names against `play_roots` and validates strictly in
    /// every build, so a test can observe the release behaviour from a debug binary.
    pub fn new_strict_with_play_roots(
        ca_cert: String,
        pem: String,
        play_roots: Vec<CertificateDer<'static>>,
    ) -> Self {
        Self {
            ca_cert,
            pem,
            play_roots: Some(play_roots),
            mode: Some(VerifierMode::Strict),
        }
    }

    /// The server's CA and this player's certificate and key, for sockets reqwest does not open.
    pub(crate) fn tls_material(&self) -> (&str, &str) {
        (&self.ca_cert, &self.pem)
    }

    fn verifier_mode(&self) -> VerifierMode {
        self.mode.unwrap_or(VerifierMode::for_build())
    }

    // Under PreferIpv4 the local socket is pinned to IPv4, which is what every
    // released version does: server domains publish both A and AAAA records, and
    // some Windows machines advertise an IPv6 stack they cannot route, so pinning
    // keeps every connection on IPv4 while still letting the connector fall back
    // across every resolved A record.
    //
    // Under PreferIpv6 the pin is omitted, which hands the choice to hyper-util's
    // Happy Eyeballs: IPv6 first, IPv4 racing 300ms behind. The pin is lifted only
    // for a host where a probe has already seen IPv6 answering, so that fallback
    // delay is the exception rather than something every request pays.
    pub fn client_builder(&self, preference: AddressFamilyPreference) -> ClientBuilder {
        let identity = BvcClientTls::identity_from_pem(&self.pem)
            .expect("this player's certificate and key must parse");
        let play_roots = match &self.play_roots {
            Some(roots) => roots.clone(),
            None => PlayRoots::builtin().expect("the built-in play roots must parse"),
        };
        let mut builder = BvcClientTls::http_client_builder_with_play_roots(
            play_roots,
            Some(&self.ca_cert),
            Some(identity),
            self.verifier_mode(),
        )
        .expect("the TLS configuration must build from the server's certificate authority")
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90));

        if matches!(preference, AddressFamilyPreference::PreferIpv4) {
            builder = builder.local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        }

        builder
    }

    pub(crate) fn get_client(&self, preference: AddressFamilyPreference) -> ReqwestClient {
        self.client_builder(preference).build().unwrap()
    }
}
