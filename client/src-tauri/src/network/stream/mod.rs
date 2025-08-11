mod stream_manager;

use crate::AudioPacket;
use crate::NetworkPacket;
use std::net::SocketAddr;
use std::sync::Arc;
use common::structs::packet::PacketOwner;
use s2n_quic::client::Connect;
use s2n_quic::Client;
use stream_manager::StreamTrait;
use stream_manager::StreamTraitType;
use std::error::Error;

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
        app_handle: tauri::AppHandle
    ) -> Self {
        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                producer.clone(),
                None,
                app_handle.clone()
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                consumer.clone(),
                None,
                None,
                app_handle.clone()
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
        key: String
    ) -> Result<(), Box<dyn Error>> {
        // Stop the current stream if we're re-initializing our new one
        self.stop().await?;

        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            cert.as_bytes().to_vec(),
            key.as_bytes().to_vec()
        ).await?;

        let client = Client::builder()
            .with_tls(provider)?
            .with_io("0.0.0.0:0")?
            .start()?;
        
        let connect = Connect::new(socket).with_server_name(server);

        let mut connection = client.connect(connect).await?;
        _ = connection.keep_alive(true);
        let stream = connection.open_bidirectional_stream().await?;
        _ = stream.connection().keep_alive(true);

        let (recv, send) = stream.split();
        _ = recv.connection().keep_alive(true);
        _ = send.connection().keep_alive(true);

        self.input = StreamTraitType::Input(
            stream_manager::InputStream::new(
                self.producer.clone(),
                Some(recv),
                self.app_handle.clone()
            )
        );

        self.output = StreamTraitType::Output(
            stream_manager::OutputStream::new(
                self.consumer.clone(),
                Some(PacketOwner {
                    name,
                    client_id: (0..32).map(|_| rand::random::<u8>()).collect()
                }),
                Some(send),
                self.app_handle.clone()
            )
        );

        self.input.start().await?;
        self.output.start().await?;
        return Ok(());
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.input.stop().await?;
        self.output.stop().await?;

        Ok(())
    }
}