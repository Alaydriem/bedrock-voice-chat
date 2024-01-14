use crate::config::ApplicationConfig;
use tokio::task::JoinHandle;
use s2n_quic::Server;
use std::{ error::Error, path::Path };
use std::sync::Arc;
use std::collections::HashMap;
use async_mutex::Mutex;
use common::{ mtlsprovider::MtlsProvider, structs::packet::QuicNetworkPacket };

/// The size our main ringbuffer can hold
const MAIN_RINGBUFFER_CAPACITY: usize = 10000;

/// The site each connection ringbuffer can hold
const CONNECTION_RINGERBUFFER_CAPACITY: usize = 1000;

pub(crate) fn get_task(
    config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>
) -> JoinHandle<()> {
    let app_config = config.clone();
    return tokio::task::spawn(async move {
        _ = server(&app_config.clone(), queue.clone()).await;
    });
}

/// A generic QUIC server to handle QuickNetworkPacket<PacketTypeTrait> packets
async fn server(
    app_config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>
) -> Result<(), Box<dyn Error>> {
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

    let main_producer_mux_for_deadqueue = main_producer_mux.clone();
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
                                    let send_client_id = client_id.clone();

                                    // Receiving Stream
                                    tokio::spawn(async move {
                                        let main_producer_mux = main_producer_mux.clone();

                                        let magic_header: Vec<u8> = vec![251, 33, 51, 0, 27];
                                        let mut packet = Vec::<u8>::new();

                                        while let Ok(Some(data)) = receive_stream.receive().await {
                                            packet.append(&mut data.to_vec());

                                            let packet_header = packet.get(0..5);

                                            let packet_header = match packet_header {
                                                Some(header) => header.to_vec(),
                                                None => {
                                                    continue;
                                                }
                                            };

                                            let packet_length = packet.get(5..13);
                                            if packet_length.is_none() {
                                                continue;
                                            }

                                            let packet_len = usize::from_be_bytes(
                                                packet_length.unwrap().try_into().unwrap()
                                            );

                                            if
                                                packet_header.eq(&magic_header) &&
                                                packet.len() == packet_len + 13
                                            {
                                                let packet_to_process = packet.clone();
                                                let packet_to_process = packet_to_process.get(
                                                    13..packet.len()
                                                );

                                                packet = Vec::<u8>::new();

                                                if packet_to_process.is_none() {
                                                    tracing::error!(
                                                        "RON serialized packet data length mismatch after verifier."
                                                    );
                                                    continue;
                                                }

                                                match
                                                    QuicNetworkPacket::from_vec(
                                                        packet_to_process.unwrap()
                                                    )
                                                {
                                                    Ok(packet) => {
                                                        let mut client_id = client_id.lock().await;
                                                        let author = packet.client_id.clone();

                                                        if client_id.is_none() {
                                                            *client_id = Some(author.clone());
                                                            tracing::debug!(
                                                                "{:?} Connected",
                                                                author
                                                            );
                                                        }

                                                        let mut main_producer_mux =
                                                            main_producer_mux.lock_arc().await;
                                                        _ = main_producer_mux.push(packet).await;
                                                    }
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "Unable to deserialize RON packet. Possible packet length issue? {}",
                                                            e.to_string()
                                                        );
                                                        continue;
                                                    }
                                                };
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
                                                    if
                                                        packet.data.broadcast() ||
                                                        client_id.ne(&author)
                                                    {
                                                        match packet.to_vec() {
                                                            Ok(rs) => {
                                                                _ = send_stream.send(
                                                                    rs.into()
                                                                ).await;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!(
                                                                    "{:?}",
                                                                    e.to_string()
                                                                );
                                                            }
                                                        };
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

            println!("Connection loop ended.");
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

    tasks.push(
        tokio::spawn(async move {
            let main_producer_mux = main_producer_mux_for_deadqueue.clone();
            let queue = queue.clone();

            #[allow(irrefutable_let_patterns)]
            while let packet = queue.pop().await {
                let mut main_producer_mux = main_producer_mux.lock_arc().await;
                _ = main_producer_mux.push(packet).await;
            }
        })
    );

    for task in tasks {
        _ = task.await;
    }
    Ok(())
}
