mod health_manager;
mod stream_manager;

use crate::AudioPacket;
use crate::NetworkPacket;
use common::s2n_quic::Client;
use common::s2n_quic::Connection;
use common::s2n_quic::client::Connect;
use common::structs::packet::PacketOwner;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use stream_manager::StreamTrait;
use stream_manager::StreamTraitType;

use health_manager::ConnectionHealthManager;

// Per-port handshake budget. A blackholed UDP port produces no response at all,
// so this timeout — not an error — is what ends the attempt and moves on to the
// next candidate.
const CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) struct NetworkStreamManager {
    producer: Arc<flume::Sender<AudioPacket>>,
    consumer: Arc<flume::Receiver<NetworkPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
    health_manager: ConnectionHealthManager,
}

impl NetworkStreamManager {
    /// Initializes the NetworkStreamManager
    /// By default, this doesn't do anything accept setup the StreamTraitTypes
    /// The stream will not start until it is connected
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        let health_manager = ConnectionHealthManager::new(app_handle.clone());

        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                producer.clone(),
                None,
                app_handle.clone(),
                health_manager.health_state(),
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                consumer.clone(),
                None,
                None,
                app_handle.clone(),
            )),
            app_handle: app_handle.clone(),
            health_manager,
        }
    }

    /// Initializes a new network connection to the server, and immediately begins
    pub async fn restart(
        &mut self,
        server_fqdn: String,
        server_url: String,
        sockets: Vec<SocketAddr>,
        name: String,
        ca_cert: String,
        cert: String,
        key: String,
    ) -> Result<(), Box<dyn Error>> {
        self.stop().await?;

        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            cert.as_bytes().to_vec(),
            key.as_bytes().to_vec(),
        )
        .await?;

        let dg_endpoint = common::s2n_quic::provider::datagram::default::Endpoint::builder()
            .with_send_capacity(1024)
            .expect("send cap > 0")
            .with_recv_capacity(1024)
            .expect("recv cap > 0")
            .build()
            .expect("build dg endpoint");

        let client = Client::builder()
            .with_tls(provider)?
            .with_io("0.0.0.0:0")?
            .with_datagram(dg_endpoint)?
            .start()?;

        let mut connection = Self::connect_first_available(&client, &sockets, &server_fqdn).await?;
        connection.keep_alive(true)?;
        let conn_arc = Arc::new(connection);
        self.health_manager.reset();

        let packet_owner = PacketOwner {
            name,
            client_id: (0..32).map(|_| rand::random::<u8>()).collect(),
        };

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
            self.health_manager.health_state(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            Some(packet_owner.clone()),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
        ));

        self.input.start().await?;
        self.output.start().await?;
        self.health_manager
            .start(conn_arc, Some(packet_owner.clone()), server_url);

        // The control plane reports as this connection's identity (the same
        // name the OutputStream stamps as packet owner). Publish it, then nudge
        // a full snapshot so a fresh player's server-side state is never empty.
        if let Some(identity) = self
            .app_handle
            .try_state::<Arc<crate::control::ConnectionIdentity>>()
        {
            identity.set(Some(packet_owner.name.clone()));
        }
        if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
            bus.self_state();
            bus.preferences();
        }

        Ok(())
    }

    // Walks the candidate ports in order and returns the first completed
    // handshake. The winning port is logged: it is the only signal separating
    // "the primary port works" from "this user is on the fallback", and that
    // distribution is what justifies maintaining the list.
    async fn connect_first_available(
        client: &Client,
        sockets: &[SocketAddr],
        server_fqdn: &str,
    ) -> Result<Connection, Box<dyn Error>> {
        let mut last_error: Option<String> = None;

        for socket in sockets {
            let connect = Connect::new(*socket).with_server_name(server_fqdn.to_string());

            match tokio::time::timeout(CONNECT_ATTEMPT_TIMEOUT, client.connect(connect)).await {
                Ok(Ok(connection)) => {
                    log::info!("QUIC handshake succeeded on {}", socket);
                    return Ok(connection);
                }
                Ok(Err(e)) => {
                    log::warn!("QUIC handshake rejected on {}: {}", socket, e);
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    log::warn!(
                        "QUIC handshake timed out on {} after {:?}",
                        socket,
                        CONNECT_ATTEMPT_TIMEOUT
                    );
                    last_error = Some(format!("timed out after {:?}", CONNECT_ATTEMPT_TIMEOUT));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| "no candidate QUIC ports were available".to_string())
            .into())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.clear_connection_identity();
        self.health_manager.stop();
        self.input.stop().await?;
        self.output.stop().await?;

        Ok(())
    }

    // A stopped stream must not keep reporting as the old connection; the
    // reporter skips reports while no identity is published.
    fn clear_connection_identity(&self) {
        if let Some(identity) = self
            .app_handle
            .try_state::<Arc<crate::control::ConnectionIdentity>>()
        {
            identity.set(None);
        }
    }

    pub async fn reset(&mut self) -> Result<(), anyhow::Error> {
        self.clear_connection_identity();
        self.health_manager.stop();
        let (_, _) = tokio::join!(self.input.stop(), self.output.stop());
        self.health_manager.reset();

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            None,
            self.app_handle.clone(),
            self.health_manager.health_state(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            None,
            None,
            self.app_handle.clone(),
        ));

        Ok(())
    }
}
