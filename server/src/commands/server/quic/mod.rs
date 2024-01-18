use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use common::mtlsprovider::MtlsProvider;
use common::structs::packet::{
    AudioFramePacket,
    DebugPacket,
    PacketType,
    PlayerDataPacket,
    QuicNetworkPacket,
    QuicNetworkPacketData,
    QUICK_NETWORK_PACKET_HEADER,
};
use moka::future::Cache;
use s2n_quic::Server;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::time::Duration;
use tokio::task::JoinHandle;
use kanal::{ AsyncSender, AsyncReceiver };

/// The size our main ringbuffer can hold
const MAIN_RINGBUFFER_CAPACITY: usize = 10000;

/// The site each connection ringbuffer can hold
const CONNECTION_RINGERBUFFER_CAPACITY: usize = 10000;

/// Player position data
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, common::Player, RandomState>>>
> = OnceCell::new();

type AsyncProducerBuffer = AsyncSender<QuicNetworkPacket>;
type AsyncConsumerBuffer = AsyncReceiver<QuicNetworkPacket>;
type MutexMapProducer = Arc<Mutex<AsyncSender<QuicNetworkPacket>>>;
type MutexMapConsumer = Arc<Mutex<AsyncReceiver<QuicNetworkPacket>>>;

pub(crate) async fn get_task(
    config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>
) -> Result<Vec<JoinHandle<()>>, anyhow::Error> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let app_config = config.clone();

    // Instantiate the player position cache
    // Player positions are valid for 5 minutes before they are evicted
    PLAYER_POSITION_CACHE.get_or_init(async {
        return Some(
            Arc::new(
                moka::future::Cache
                    ::builder()
                    .time_to_live(Duration::from_secs(300))
                    .max_capacity(100)
                    .build()
            )
        );
    }).await;

    // This is our main ring buffer. Incoming packets from any client are added to it
    // Then a separate thread pushes them to all connected clients, which is a separate ringbuffer
    let (main_producer, main_consumer) =
        kanal::bounded_async::<QuicNetworkPacket>(MAIN_RINGBUFFER_CAPACITY);

    let cache = match get_cache().await {
        Ok(cache) => cache,
        Err(e) => {
            tracing::error!(
                "A cache was constructed but didn't survive. This shouldn't happen. Restart bvc server."
            );
            return Err(e);
        }
    };

    let cache_c = cache.clone();

    let main_producer_mux = Arc::new(Mutex::<AsyncProducerBuffer>::new(main_producer));
    let main_consumer_mux = Arc::new(Mutex::<AsyncConsumerBuffer>::new(main_consumer));
    let main_producer_mux_for_deadqueue = main_producer_mux.clone();
    let mutex_map = Arc::new(Mutex::new(HashMap::<u64, MutexMapProducer>::new()));
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

    let outer_thread_shutdown = shutdown.clone();
    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            tracing::info!("Started QUIC listening server.");
            let mut connection_id: Option<u64> = None;

            while let Some(mut connection) = server.accept().await {
                connection_id = Some(connection.id());
                let cache = cache.clone();
                let main_producer_mux = main_producer_mux.clone();

                // Create the mutex map when the stream is created
                let mut mutex_map = mutex_map.lock_arc().await;

                let (producer, consumer) = kanal::bounded_async::<QuicNetworkPacket>(
                    CONNECTION_RINGERBUFFER_CAPACITY
                );

                let actual_consumer = Arc::new(Mutex::new(consumer));

                mutex_map.insert(connection.id(), Arc::new(Mutex::new(producer)));

                // Each connection sits in it's own thread
                let cache = cache.clone();
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
                                let i1_shutdown = shutdown.clone();
                                tokio::spawn(async move {
                                    let shutdown = i1_shutdown.clone();
                                    let main_producer_mux = main_producer_mux.clone();

                                    let magic_header: Vec<u8> =
                                        QUICK_NETWORK_PACKET_HEADER.to_vec();
                                    let mut packet = Vec::<u8>::new();

                                    while let Ok(Some(data)) = receive_stream.receive().await {
                                        if shutdown.load(Ordering::Relaxed) {
                                            _ = receive_stream.stop_sending((204u8).into());
                                            tracing::info!("Receiving stream was ended.");
                                            break;
                                        }
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

                                        // If the current packet starts with the magic header and we have enough bytes, drain it
                                        if
                                            packet_header.eq(&magic_header) &&
                                            packet.len() >= packet_len + 13
                                        {
                                            let packet_to_process = packet
                                                .get(0..packet_len + 13)
                                                .unwrap()
                                                .to_vec();

                                            let mut remaining_data = packet
                                                .get(packet_len + 13..packet.len())
                                                .unwrap()
                                                .to_vec()
                                                .into_boxed_slice()
                                                .to_vec();
                                            packet = vec![0; 0];
                                            packet.append(&mut remaining_data);
                                            packet.shrink_to(packet.len());
                                            packet.truncate(packet.len());

                                            // Strip the header and frame length
                                            let packet_to_process = packet_to_process
                                                .get(13..packet_to_process.len())
                                                .unwrap();

                                            match QuicNetworkPacket::from_vec(&packet_to_process) {
                                                Ok(packet) => {
                                                    let mut client_id = client_id.lock().await;
                                                    let author = packet.client_id.clone();

                                                    if client_id.is_none() {
                                                        *client_id = Some(author.clone());
                                                        tracing::debug!("{:?} Connected", author);
                                                    }

                                                    let main_producer_mux =
                                                        main_producer_mux.lock_arc().await;
                                                    _ = main_producer_mux.send(packet).await;
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
                                let i2_shutdown = shutdown.clone();
                                tokio::spawn(async move {
                                    let shutdown = i2_shutdown.clone();
                                    let cache = cache.clone();
                                    let consumer = consumer.clone();
                                    let consumer = consumer.lock_arc().await;
                                    let client_id = send_client_id.clone();

                                    #[allow(irrefutable_let_patterns)]
                                    while let packet = consumer.recv().await {
                                        if shutdown.load(Ordering::Relaxed) {
                                            // Flush and finish any data currently in the buffer then close the stream
                                            _ = send_stream.close().await;
                                            tracing::info!("Sending stream was ended.");
                                            break;
                                        }
                                        match packet {
                                            Ok(mut packet) => {
                                                let author = Some(packet.client_id.clone());
                                                let client_id = client_id.lock().await;

                                                match packet.packet_type {
                                                    PacketType::AudioFrame => {
                                                        match packet.get_data() {
                                                            Some(data) => {
                                                                let data: Result<
                                                                    AudioFramePacket,
                                                                    ()
                                                                > = data.try_into();

                                                                // If we don't have coordinates on this audio frame, add them from the cache
                                                                match data {
                                                                    Ok(mut data) =>
                                                                        match data.coordinate {
                                                                            Some(_) => {}
                                                                            None =>
                                                                                match
                                                                                    cache.get(
                                                                                        &data.author
                                                                                    ).await
                                                                                {
                                                                                    Some(
                                                                                        position,
                                                                                    ) => {
                                                                                        data.coordinate =
                                                                                            Some(
                                                                                                position.coordinates
                                                                                            );
                                                                                        packet.data =
                                                                                            QuicNetworkPacketData::AudioFrame(
                                                                                                data
                                                                                            );
                                                                                    }
                                                                                    None => {}
                                                                                }
                                                                        }
                                                                    Err(_) => {}
                                                                }
                                                            }
                                                            None => {}
                                                        }
                                                    }
                                                    _ => {}
                                                }

                                                // Send the packet to the player if it's a broadcast packet, or if it wasn't originated by them
                                                if packet.broadcast() || client_id.ne(&author) {
                                                    match packet.to_vec() {
                                                        Ok(rs) => {
                                                            _ = send_stream.send(rs.into()).await;
                                                            _ = send_stream.flush().await;
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("{:?}", e.to_string());
                                                        }
                                                    };
                                                }
                                            }
                                            Err(e) => {}
                                        }
                                    }

                                    tracing::info!("Sending stream died.");
                                });
                            }
                            None => {}
                        }
                    Err(e) => {
                        tracing::error!("{:?}", e.to_string());
                    }
                }
            }

            // If the main loop ends, then the connection was dropped
            // And we need to clean up the mutex_map table so we aren't keeping connection_ids that have been previously dropped
            match connection_id {
                Some(connection_id) => {
                    let mut mutex_map = mutex_map.lock().await;
                    mutex_map.remove(&connection_id);
                    tracing::info!("Connection {} dropped", connection_id);
                }
                None => {}
            }
        })
    );

    // Iterate through the main consumer mutex and broadcast it to all active connections
    let t2_shutdown = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            let shutdown = t2_shutdown.clone();
            let main_consumer_mux = main_consumer_mux.lock_arc().await;
            let mutex_map = processor_mutex_map.clone();

            // Extract the data from the main mux, then push it into everyone elses private mux
            #[allow(irrefutable_let_patterns)]
            while let packet = main_consumer_mux.recv().await {
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Broadcast QUIC thread was ended.");
                    break;
                }
                match packet {
                    Ok(packet) => {
                        let mutex_map = mutex_map.lock_arc().await;
                        for (_, producer) in mutex_map.clone().into_iter() {
                            let producer = producer.lock_arc().await;
                            _ = producer.send(packet.clone()).await;
                        }
                    }
                    Err(_) => {}
                }
            }
        })
    );

    let t3_shutdown = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            let shutdown = t3_shutdown.clone();
            let cache = cache_c.clone();
            let main_producer_mux = main_producer_mux_for_deadqueue.clone();
            let queue = queue.clone();

            #[allow(irrefutable_let_patterns)]
            while let packet = queue.pop().await {
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Webhook QUIC thread ended.");
                    break;
                }

                let pkc = packet.clone();

                // Packet specific handling
                match pkc.packet_type {
                    PacketType::AudioFrame => {}
                    PacketType::Debug => {}
                    PacketType::PlayerData => {
                        match packet.get_data() {
                            Some(data) => {
                                let data: Result<PlayerDataPacket, ()> = data.try_into();
                                match data {
                                    Ok(data) => {
                                        for player in data.players.clone() {
                                            cache.insert(player.name.clone(), player.clone()).await;
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            None => {}
                        }
                    }
                }

                // Push all packets into the main mux if they're received.
                let main_producer_mux = main_producer_mux.lock_arc().await;
                _ = main_producer_mux.send(packet).await;
            }
        })
    );

    let shutdown_monitor = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            tracing::info!("Listening for CTRL+C");
            let shutdown = shutdown_monitor.clone();
            _ = tokio::signal::ctrl_c().await;
            shutdown.store(true, Ordering::Relaxed);
            tracing::info!("Shutdown signal received, signaling QUIC threads to terminate.");
        })
    );

    return Ok(tasks);
}

/// Returns the cache object without if match branching nonsense
async fn get_cache() -> Result<Arc<Cache<String, common::Player>>, anyhow::Error> {
    match PLAYER_POSITION_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => Ok(cache.clone()),
                None => Err(anyhow!("Cache not found.")),
            }

        None => Err(anyhow!("Cache not found")),
    }
}
