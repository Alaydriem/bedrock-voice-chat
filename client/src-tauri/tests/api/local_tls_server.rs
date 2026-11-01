use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;

use bvc_client_lib::ApiClient;
use common::structs::reachability::AddressFamilyPreference;
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use super::play_chain::PlayChain;

/// A TLS server on loopback that answers every request with 200.
pub struct LocalTlsServer;

impl LocalTlsServer {
    /// Serves `chain` and returns the port.
    pub fn start(chain: PlayChain) -> u16 {
        let config = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::aws_lc_rs::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![chain.leaf], chain.leaf_key)
            .unwrap(),
        );
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            for tcp in listener.incoming().flatten() {
                let connection = ServerConnection::new(config.clone()).unwrap();
                let mut tls = StreamOwned::new(connection, tcp);
                let mut request = [0u8; 4096];
                if tls.read(&mut request).is_ok() {
                    let _ = tls.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    );
                    let _ = tls.flush();
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                }
            }
        });

        port
    }

    /// `GET https://{name}:{port}/` through the API client, with `name` resolved to loopback.
    pub async fn get(
        client: &ApiClient,
        name: &str,
        port: u16,
    ) -> Result<u16, common::reqwest::Error> {
        let http = client
            .client_builder(AddressFamilyPreference::PreferIpv4)
            .resolve(name, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
            .build()
            .unwrap();

        http.get(format!("https://{name}:{port}/"))
            .send()
            .await
            .map(|response| response.status().as_u16())
    }
}
