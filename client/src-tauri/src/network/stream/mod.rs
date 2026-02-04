mod health_manager;
mod stream_manager;

use crate::AudioPacket;
use crate::NetworkPacket;
use common::structs::packet::PacketOwner;
use common::s2n_quic::client::Connect;
use common::s2n_quic::Client;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use stream_manager::StreamTrait;
use stream_manager::StreamTraitType;

pub use common::structs::network::ConnectionHealth;
use health_manager::ConnectionHealthManager;

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
        socket: SocketAddr,
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

        let connect = Connect::new(socket).with_server_name(server_fqdn);

        let mut connection = client.connect(connect).await?;
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
            .start(conn_arc, Some(packet_owner), server_url);

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.health_manager.stop();
        self.input.stop().await?;
        self.output.stop().await?;

        Ok(())
    }

    pub async fn reset(&mut self) -> Result<(), anyhow::Error> {
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
