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

pub(crate) struct NetworkStreamManager {
    producer: Arc<flume::Sender<AudioPacket>>,
    consumer: Arc<flume::Receiver<NetworkPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
}

impl NetworkStreamManager {
    /// Initializes the NetworkStreamManager
    /// By default, this doesn't do anything accept setup the StreamTraitTypes
    /// The stream will not start until it is connected
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>, // Sends data to audio output stream
        consumer: Arc<flume::Receiver<NetworkPacket>>, // Recv from the audio input stream
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                producer.clone(),
                None,
                app_handle.clone(),
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                consumer.clone(),
                None,
                None,
                app_handle.clone(),
            )),
            app_handle: app_handle.clone(),
        }
    }

    /// Initializes a new network connection to the server, and immediately begins
    pub async fn restart(
        &mut self,
        server: String,
        socket: SocketAddr,
        name: String,
        ca_cert: String,
        cert: String,
        key: String,
    ) -> Result<(), Box<dyn Error>> {
        // Stop the current stream if we're re-initializing our new one
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

        let connect = Connect::new(socket).with_server_name(server);

        let mut connection = client.connect(connect).await?;
        connection.keep_alive(true)?;
        let conn_arc = Arc::new(connection);

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            Some(PacketOwner {
                name,
                client_id: (0..32).map(|_| rand::random::<u8>()).collect(),
            }),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
        ));

        self.input.start().await?;
        self.output.start().await?;
        return Ok(());
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.input.stop().await?;
        self.output.stop().await?;

        Ok(())
    }

    /// Resets the network stream manager by stopping all streams and recreating them
    pub async fn reset(&mut self) -> Result<(), anyhow::Error> {
        // Stop both streams concurrently
        let (_, _) = tokio::join!(
            self.input.stop(),
            self.output.stop()
        );

        // Recreate streams without connection (will be reconnected later)
        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            None,
            self.app_handle.clone(),
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
