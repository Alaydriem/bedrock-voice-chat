use crate::config::ApplicationConfig;
use tokio::task::JoinHandle;
use s2n_quic::Server;
use std::{ error::Error, path::Path };
use std::sync::Arc;
use std::collections::HashMap;
use async_mutex::Mutex;
use rustls::{ cipher_suite, ClientConfig, RootCertStore, ServerConfig, SupportedCipherSuite };
use s2n_quic::provider::{ tls, tls::rustls::rustls };
use std::io::Cursor;
use tokio::io::AsyncReadExt;
use common::structs::packet::QuicNetworkPacket;

/// The size our main ringbuffer can hold
const MAIN_RINGBUFFER_CAPACITY: usize = 10000;

/// The site each connection ringbuffer can hold
const CONNECTION_RINGERBUFFER_CAPACITY: usize = 1000;

pub(crate) fn get_task(config: &ApplicationConfig) -> JoinHandle<()> {
    let app_config = config.clone();
    return tokio::task::spawn(async move {
        _ = server(&app_config.clone()).await;
    });
}

/// A generic QUIC server to handle QuickNetworkPacket<PacketTypeTrait> packets
async fn server(app_config: &ApplicationConfig) -> Result<(), Box<dyn Error>> {
    // This is our main ring buffer. Incoming packets from any client are added to it
    // Then a separate thread pushes them to all connected clients, which is a separate ringbuffer
    let ring = async_ringbuf::AsyncHeapRb::<QuicNetworkPacket>::new(MAIN_RINGBUFFER_CAPACITY);

    let (main_producer, main_consumer) = ring.split();

    let main_producer_mux = Arc::new(
        Mutex::<
            async_ringbuf::AsyncProducer<
                QuicNetworkPacket,
                Arc<
                    async_ringbuf::AsyncRb<
                        QuicNetworkPacket,
                        ringbuf::SharedRb<
                            QuicNetworkPacket,
                            Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                        >
                    >
                >
            >
        >::new(main_producer)
    );

    let main_consumer_mux = Arc::new(
        Mutex::<
            async_ringbuf::AsyncConsumer<
                QuicNetworkPacket,
                Arc<
                    async_ringbuf::AsyncRb<
                        QuicNetworkPacket,
                        ringbuf::SharedRb<
                            QuicNetworkPacket,
                            Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                        >
                    >
                >
            >
        >::new(main_consumer)
    );

    let mutex_map = Arc::new(
        Mutex::new(
            HashMap::<
                u64,
                Arc<
                    Mutex<
                        async_ringbuf::AsyncProducer<
                            QuicNetworkPacket,
                            Arc<
                                async_ringbuf::AsyncRb<
                                    QuicNetworkPacket,
                                    ringbuf::SharedRb<
                                        QuicNetworkPacket,
                                        Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                                    >
                                >
                            >
                        >
                    >
                >
            >::new()
        )
    );

    let processor_mutex_map = mutex_map.clone();

    // Setups mTLS for the connection
    let cert_path = format!("{}/ca.crt", &app_config.server.tls.certs_path.clone());
    let key_path = format!("{}/ca.key", &app_config.server.tls.certs_path.clone());

    let cert = Path::new(&cert_path);
    let key = Path::new(&key_path);

    let provider = MtlsProvider::new(cert, cert, key).await?;

    let io_connection_str: &str = &format!(
        "{}:{}",
        &app_config.server.listen,
        &app_config.server.quic_port
    );

    tracing::info!("Starting QUIC server with CA: {} on {}", &cert_path, io_connection_str);
    // Initialize the server
    let mut server = Server::builder()
        .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
        .with_tls(provider)?
        .with_io(io_connection_str)?
        .start()?;

    let mut tasks = Vec::new();

    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            tracing::info!("Started QUIC listening server.");
            let mut connection_id: Option<u64> = None;

            while let Some(mut connection) = server.accept().await {
                connection_id = Some(connection.id());
                let main_producer_mux = main_producer_mux.clone();

                // Create the mutex map when the stream is created
                let mut mutex_map = mutex_map.lock_arc().await;

                let ring = async_ringbuf::AsyncHeapRb::<QuicNetworkPacket>::new(
                    CONNECTION_RINGERBUFFER_CAPACITY
                );

                let (producer, consumer) = ring.split();

                let actual_consumer = Arc::new(
                    Mutex::<
                        async_ringbuf::AsyncConsumer<
                            QuicNetworkPacket,
                            Arc<
                                async_ringbuf::AsyncRb<
                                    QuicNetworkPacket,
                                    ringbuf::SharedRb<
                                        QuicNetworkPacket,
                                        Vec<std::mem::MaybeUninit<QuicNetworkPacket>>
                                    >
                                >
                            >
                        >
                    >::new(consumer)
                );

                mutex_map.insert(connection.id(), Arc::new(Mutex::new(producer)));

                // Each connection sits in it's own thread
                tokio::spawn(async move {
                    let main_producer_mux = main_producer_mux.clone();

                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    // When a client connects, store the author property on the QuicNetworkPacket for this stream
                                    // This enables us to identify packets directed to us.
                                    let cid: Option<Vec<u8>> = None;
                                    let client_id = Arc::new(Mutex::new(cid));
                                    let rec_client_id = client_id.clone();
                                    let send_client_id = client_id.clone();

                                    // Receiving Stream
                                    tokio::spawn(async move {
                                        let main_producer_mux = main_producer_mux.clone();

                                        while let Ok(Some(data)) = receive_stream.receive().await {
                                            let client_id = rec_client_id.clone();
                                            // Take the data packet, and push it into the main mux
                                            let d = data.clone();
                                            let ds = std::str::from_utf8(&d).unwrap();
                                            match ron::from_str::<QuicNetworkPacket>(ds) {
                                                Ok(packet) => {
                                                    let mut client_id = client_id.lock().await;
                                                    let author = packet.client_id.clone();
                                                    if client_id.is_none() {
                                                        *client_id = Some(author.clone());
                                                        tracing::debug!("{:?} Connected", author);
                                                    }

                                                    let mut main_producer_mux =
                                                        main_producer_mux.lock_arc().await;
                                                    _ = main_producer_mux.push(packet).await;
                                                }
                                                Err(_e) => {}
                                            }
                                        }
                                    });

                                    // Sending Stream
                                    let consumer = actual_consumer.clone();
                                    tokio::spawn(async move {
                                        let consumer = consumer.clone();
                                        let mut consumer = consumer.lock_arc().await;
                                        let client_id = send_client_id.clone();

                                        #[allow(irrefutable_let_patterns)]
                                        while let packet = consumer.pop().await {
                                            match packet {
                                                Some(packet) => {
                                                    let author = Some(packet.client_id.clone());
                                                    let client_id = client_id.lock().await;

                                                    // If the packet ID is
                                                    if client_id.ne(&author) {
                                                        match ron::to_string(&packet) {
                                                            Ok::<String, _>(rs) => {
                                                                let reader = rs.as_bytes().to_vec();

                                                                // Send the data, then flush the buffer
                                                                _ = send_stream.send(
                                                                    reader.into()
                                                                ).await;
                                                                _ = send_stream.flush().await;
                                                            }
                                                            Err(_) => {}
                                                        }
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                    });
                                }
                                None => {}
                            }
                        Err(_) => {}
                    }
                });
            }

            // If the main loop ends, then the connection was dropped
            // And we need to clean up the mutex_map table so we aren't keeping connection_ids that have been previously dropped
            match connection_id {
                Some(connection_id) => {
                    let mut mutex_map = mutex_map.lock().await;
                    mutex_map.remove(&connection_id);
                    tracing::trace!("Connection {} dropped", connection_id);
                }
                None => {}
            }
        })
    );

    // Iterate through the main consumer mutex and broadcast it to all active connections
    tasks.push(
        tokio::spawn(async move {
            let mut main_consumer_mux = main_consumer_mux.lock_arc().await;
            let mutex_map = processor_mutex_map.clone();

            // Extract the data from the main mux, then push it into everyone elses private mux
            #[allow(irrefutable_let_patterns)]
            while let packet = main_consumer_mux.pop().await {
                match packet {
                    Some(packet) => {
                        let mutex_map = mutex_map.lock_arc().await;
                        for (_, producer) in mutex_map.clone().into_iter() {
                            let mut producer = producer.lock_arc().await;
                            _ = producer.push(packet.clone()).await;
                        }
                    }
                    None => {}
                }
            }
        })
    );

    for task in tasks {
        _ = task.await;
    }
    Ok(())
}

static PROTOCOL_VERSIONS: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS13];
static DEFAULT_CIPHERSUITES: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
];

struct MtlsProvider {
    root_store: rustls::RootCertStore,
    my_cert_chain: Vec<rustls::Certificate>,
    my_private_key: rustls::PrivateKey,
}

impl tls::Provider for MtlsProvider {
    type Server = tls::rustls::Server;
    type Client = tls::rustls::Client;
    type Error = rustls::Error;

    fn start_server(self) -> Result<Self::Server, Self::Error> {
        let verifier = rustls::server::AllowAnyAuthenticatedClient::new(self.root_store);
        let mut cfg = ServerConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHERSUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(PROTOCOL_VERSIONS)?
            .with_client_cert_verifier(Arc::new(verifier))
            .with_single_cert(self.my_cert_chain, self.my_private_key.clone())?;

        cfg.max_fragment_size = None;
        cfg.alpn_protocols = vec![b"h3".to_vec()];
        Ok(cfg.into())
    }

    fn start_client(self) -> Result<Self::Client, Self::Error> {
        let mut cfg = ClientConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHERSUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(PROTOCOL_VERSIONS)?
            .with_root_certificates(self.root_store)
            .with_client_auth_cert(self.my_cert_chain, self.my_private_key)?;

        cfg.max_fragment_size = None;
        cfg.enable_early_data = true;
        cfg.alpn_protocols = vec![b"h3".to_vec()];
        Ok(cfg.into())
    }
}

impl MtlsProvider {
    pub async fn new<A: AsRef<Path>, B: AsRef<Path>, C: AsRef<Path>>(
        ca_cert_pem: A,
        my_cert_pem: B,
        my_key_pem: C
    ) -> Result<Self, rustls::Error> {
        let root_store = into_root_store(ca_cert_pem.as_ref()).await?;
        let cert_chain = into_certificate(my_cert_pem.as_ref()).await?;
        let private_key = into_private_key(my_key_pem.as_ref()).await?;
        Ok(MtlsProvider {
            root_store,
            my_cert_chain: cert_chain.into_iter().map(rustls::Certificate).collect(),
            my_private_key: rustls::PrivateKey(private_key),
        })
    }
}

async fn into_certificate(path: &Path) -> Result<Vec<Vec<u8>>, rustls::Error> {
    let mut f = tokio::fs::File
        ::open(path).await
        .map_err(|e| rustls::Error::General(format!("Failed to load file: {}", e)))?;
    let mut buf = Vec::new();
    f
        .read_to_end(&mut buf).await
        .map_err(|e| rustls::Error::General(format!("Failed to read file: {}", e)))?;
    let mut cursor = Cursor::new(buf);
    let certs = rustls_pemfile
        ::certs(&mut cursor)
        .map(|certs| certs.into_iter().collect())
        .map_err(|_| rustls::Error::General("Could not read certificate".to_string()))?;
    Ok(certs)
}

async fn into_root_store(path: &Path) -> Result<RootCertStore, rustls::Error> {
    let ca_certs = into_certificate(path).await?;
    let mut cert_store = RootCertStore::empty();
    cert_store.add_parsable_certificates(ca_certs.as_slice());
    Ok(cert_store)
}

async fn into_private_key(path: &Path) -> Result<Vec<u8>, rustls::Error> {
    let mut f = tokio::fs::File
        ::open(path).await
        .map_err(|e| rustls::Error::General(format!("Failed to load file: {}", e)))?;
    let mut buf = Vec::new();
    f
        .read_to_end(&mut buf).await
        .map_err(|e| rustls::Error::General(format!("Failed to read file: {}", e)))?;
    let mut cursor = Cursor::new(buf);

    cursor.set_position(0);

    match rustls_pemfile::pkcs8_private_keys(&mut cursor) {
        Ok(keys) if keys.is_empty() => {}
        Ok(mut keys) if keys.len() == 1 => {
            return Ok(rustls::PrivateKey(keys.pop().unwrap()).0);
        }
        Ok(keys) => {
            return Err(
                rustls::Error::General(
                    format!("Unexpected number of keys: {} (only 1 supported)", keys.len())
                )
            );
        }
        // try the next parser
        Err(_) => {}
    }
    Err(rustls::Error::General("could not load any valid private keys".to_string()))
}
