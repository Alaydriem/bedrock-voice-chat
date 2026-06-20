#![allow(dead_code)]

mod proxy_player;

use std::collections::HashMap;
use std::time::Duration;

use bedrock_client::BedrockClient;
use common::bedrock_protocol::AuthInfo;
use common::bedrock_protocol::version::ProtocolVersion;
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
    pub async fn boot(version: ProtocolVersion, names: &[&str]) -> Self {
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
                Duration::from_secs(10),
            )
            .expect("proxy started");

            // The downstream client connect triggers Proxy::accept(), which makes the
            // proxy dial the fake upstream. These two steps are interdependent: run the
            // connect and the upstream accept concurrently so neither blocks the other.
            let proxy_addr: std::net::SocketAddr =
                format!("127.0.0.1:{listen}").parse().expect("proxy addr");
            let connect = BedrockClient::connect(proxy_addr, AuthInfo::offline(name), version);
            let accept = upstream.accept_player();
            let (downstream_res, accepted) = tokio::join!(connect, accept);
            let downstream = downstream_res.expect("downstream connects to proxy");
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
