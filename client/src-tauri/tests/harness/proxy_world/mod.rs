#![allow(dead_code)]

mod proxy_player;

use std::collections::HashMap;
use std::time::Duration;

use bedrock_client::BedrockClient;
use common::bedrock_protocol::AuthInfo;
use common::bedrock_protocol::version::ProtocolVersion;
use common::structs::bedrock::AddonMode;
use tempfile::TempDir;

use crate::harness::client_proc::ClientProc;
use crate::harness::proxy_driver::FakeBedrockUpstream;
use crate::harness::server::EmbeddedServer;

pub use proxy_player::ProxyPlayer;

pub struct ProxyWorld {
    pub server: EmbeddedServer,
    pub upstream: FakeBedrockUpstream,
    pub players: HashMap<String, ProxyPlayer>,
    pub version: ProtocolVersion,
    _data_dir: TempDir,
}

impl ProxyWorld {
    /// How long the downstream connect and upstream accept get before the
    /// harness gives up.
    ///
    /// This bound only catches a *silent* hang. A connect that fails outright
    /// panics the moment it does, so the common case -- the proxy bound an
    /// address family the harness cannot reach -- is reported in seconds no
    /// matter what this is set to.
    ///
    /// So it is set generously rather than tightly. At 45s it failed a test
    /// that had passed moments earlier on a less loaded machine, which trades
    /// one flaky failure mode for another. It stays under nextest's 180s so
    /// the message below is what the log shows, instead of an opaque
    /// termination.
    const ATTACH_TIMEOUT: Duration = Duration::from_secs(120);

    /// Drive the downstream connect and the upstream accept together,
    /// failing fast and loudly on either.
    ///
    /// The proxy only dials the upstream once a downstream client has
    /// connected, so neither side can be awaited to completion before the
    /// other is started. Both failure modes used to present identically as a
    /// test that simply never finished.
    async fn connect_and_accept(
        proxy_addr: std::net::SocketAddr,
        name: &str,
        version: ProtocolVersion,
        upstream: &mut FakeBedrockUpstream,
    ) -> (bedrock_client::ClientConnection, String) {
        let connect =
            BedrockClient::connect_nethernet(proxy_addr, AuthInfo::offline(name), version);
        let accept = upstream.accept_player();
        tokio::pin!(connect);
        tokio::pin!(accept);

        let mut downstream = None;
        let mut accepted = None;

        let both = async {
            while downstream.is_none() || accepted.is_none() {
                tokio::select! {
                    result = &mut connect, if downstream.is_none() => match result {
                        Ok(conn) => downstream = Some(conn),
                        Err(e) => panic!(
                            "downstream connect to {proxy_addr} failed: {e}\n\
                             The proxy serves NetherNet signaling over TCP at that \
                             address. A connection refusal usually means it bound a \
                             different address family than the one dialed here."
                        ),
                    },
                    who = &mut accept, if accepted.is_none() => accepted = Some(who),
                }
            }
        };

        if tokio::time::timeout(Self::ATTACH_TIMEOUT, both)
            .await
            .is_err()
        {
            panic!(
                "timed out attaching {name} after {:?}: downstream connected = {}, \
                 upstream accepted = {}. Whichever is false is the side that hung.",
                Self::ATTACH_TIMEOUT,
                downstream.is_some(),
                accepted.is_some()
            );
        }

        (
            downstream.expect("downstream set before the loop exits"),
            accepted.expect("accepted set before the loop exits"),
        )
    }

    pub async fn boot(version: ProtocolVersion, names: &[&str]) -> Self {
        Self::boot_with_mode(version, names, AddonMode::NoNet).await
    }

    pub async fn boot_with_mode(
        version: ProtocolVersion,
        names: &[&str],
        addon_mode: AddonMode,
    ) -> Self {
        let data_dir = tempfile::tempdir().expect("temp data dir");
        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
        let certs_path = data_dir.path().join("certificates");
        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
        let url = format!("https://127.0.0.1:{}", server.rocket_port());

        let mut upstream = FakeBedrockUpstream::bind(version).await;
        let upstream_addr = upstream.addr();
        let mut players = HashMap::new();

        for &name in names {
            let code = server.login_code(name);
            // Empty channel name — proximity is positional, no channel join needed.
            let proc = ClientProc::spawn(name, &code, &url, "");
            proc.await_connected(Duration::from_secs(30))
                .expect("voice connect");

            let listen = EmbeddedServer::free_port_udp();
            proc.start_proxy(
                &upstream_addr.ip().to_string(),
                upstream_addr.port(),
                listen,
                Some(addon_mode),
                Duration::from_secs(10),
            )
            .expect("proxy started");

            // The downstream client connect triggers Proxy::accept(), which makes the
            // proxy dial the fake upstream. These two steps are interdependent: run the
            // connect and the upstream accept concurrently so neither blocks the other.
            let proxy_addr: std::net::SocketAddr =
                format!("127.0.0.1:{listen}").parse().expect("proxy addr");
            // NetherNet, matching the frontend the proxy now serves. A
            // RakNet dial cannot reach it: a NetherNet frontend binds no
            // RakNet listener, so a RakNet dial here fails to connect.
            // `proxy_addr` is the HTTP signaling endpoint.
            //
            // These two steps are interdependent, so they run concurrently --
            // but a plain `join!` waits for *both*, and the upstream accept
            // never happens if the downstream connect fails. That turned a
            // one-line connection error into a 180s nextest timeout with
            // nothing in the log to explain it. `select!` reports the connect
            // error the moment it happens instead.
            let (downstream, accepted) =
                Self::connect_and_accept(proxy_addr, name, version, &mut upstream).await;
            assert_eq!(
                accepted, name,
                "upstream connection identity must match actor"
            );

            upstream.start_game(name).await;

            players.insert(
                name.to_string(),
                ProxyPlayer {
                    proc,
                    _downstream: downstream,
                },
            );
        }

        Self {
            server,
            upstream,
            players,
            version,
            _data_dir: data_dir,
        }
    }

    pub fn proc(&self, name: &str) -> &ClientProc {
        &self.players.get(name).expect("known player").proc
    }

    pub fn shutdown(self) {
        for (_n, p) in self.players {
            p.proc.shutdown();
        }
    }
}
