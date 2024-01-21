use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use common::mtlsprovider::MtlsProvider;
use common::structs::packet::{
    AudioFramePacket,
    PacketType,
    PlayerDataPacket,
    QuicNetworkPacket,
    QuicNetworkPacketCollection,
    QuicNetworkPacketData,
    QUICK_NETWORK_PACKET_HEADER,
};
use kanal::{ AsyncReceiver, AsyncSender };
use moka::future::Cache;
use s2n_quic::Server;
use std::borrow::BorrowMut;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use rtrb::{ RingBuffer, Producer, Consumer };
/// The size our main ringbuffer can hold
const MAIN_RINGBUFFER_CAPACITY: usize = 10000;

/// The site each connection ringbuffer can hold
const CONNECTION_RINGERBUFFER_CAPACITY: usize = 10000;

/// Player position data
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, common::Player, RandomState>>>
> = OnceCell::new();

type AsyncProducerBuffer = AsyncSender<QuicNetworkPacketCollection>;
type AsyncConsumerBuffer = AsyncReceiver<QuicNetworkPacketCollection>;
type MutexMapProducer = Producer<QuicNetworkPacket>;
type MutexMapConsumer = Consumer<QuicNetworkPacket>;

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
    let monitor_cache = cache.clone();

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

    let producer_collection = Arc::new(
        Mutex::new(HashMap::<u64, Arc<Mutex<MutexMapProducer>>>::new())
    );
    let consumer_collection = Arc::new(Mutex::new(HashMap::<u64, MutexMapConsumer>::new()));
    let monitor_consumer_collection = consumer_collection.clone();

    let sender_collection = Arc::new(
        Mutex::new(HashMap::<u64, Arc<Mutex<AsyncProducerBuffer>>>::new())
    );
    let receiver_collection = Arc::new(
        Mutex::new(HashMap::<u64, Arc<Mutex<AsyncConsumerBuffer>>>::new())
    );
    let monitor_sender_collection = sender_collection.clone();

    let mut tasks = Vec::new();
    let outer_thread_shutdown = shutdown.clone();
    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let producer_collection = producer_collection.clone();
            let consumer_collection = consumer_collection.clone();
            let sender_collection = sender_collection.clone();
            let receiver_collection = receiver_collection.clone();

            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            tracing::info!("Started QUIC listening server.");

            while let Some(mut connection) = server.accept().await {
                // Maintain the connection
                _ = connection.keep_alive(true);
                let sender_collection = sender_collection.clone();
                let receiver_collection = receiver_collection.clone();
                let producer_collection = producer_collection.clone();
                let consumer_collection = consumer_collection.clone();
                let cache = cache.clone();

                let shutdown = shutdown.clone();
                let raw_connection_id = connection.id();

                let (sender, receiver) =
                    kanal::bounded_async::<QuicNetworkPacketCollection>(MAIN_RINGBUFFER_CAPACITY);

                _ = sender_collection
                    .clone()
                    .lock_arc().await
                    .insert(raw_connection_id, Arc::new(Mutex::new(sender)));
                receiver_collection
                    .clone()
                    .lock_arc().await
                    .insert(raw_connection_id, Arc::new(Mutex::new(receiver)));

                let (producer, consumer) = RingBuffer::<QuicNetworkPacket>::new(
                    CONNECTION_RINGERBUFFER_CAPACITY
                );

                _ = producer_collection
                    .clone()
                    .lock_arc().await
                    .insert(raw_connection_id, Arc::new(Mutex::new(producer)));
                _ = consumer_collection
                    .clone()
                    .lock_arc().await
                    .insert(raw_connection_id, consumer);

                tokio::spawn(async move {
                    let receiver_collection = receiver_collection.clone();
                    let producer_collection = producer_collection.clone();
                    let sender_collection = sender_collection.clone();
                    let consumer_collection = consumer_collection.clone();
                    let cache = cache.clone();
                    let stream_cache = cache.clone();

                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    // When a client connects, store the author property on the QuicNetworkPacket for this stream
                                    // This enables us to identify packets directed to us.
                                    let cid: Option<Vec<u8>> = None;
                                    let client_id = Arc::new(Mutex::new(cid));

                                    let receiver_shutdown = shutdown.clone();
                                    let receiver_cache = stream_cache.clone();

                                    let mut tasks = Vec::new();

                                    let disconnected = Arc::new(AtomicBool::new(false));
                                    let receiver_disconnected = disconnected.clone();
                                    let sender_disconnected = disconnected.clone();

                                    let outer_receiver_collection = receiver_collection.clone();
                                    let outer_producer_collection = producer_collection.clone();
                                    let outer_sender_collection = sender_collection.clone();
                                    let outer_consumer_collection = consumer_collection.clone();

                                    // Receiving Stream
                                    tasks.push(
                                        tokio::spawn(async move {
                                            let disconnected = receiver_disconnected.clone();
                                            let producer_collection = producer_collection.clone();
                                            let mut producer = producer_collection
                                                .lock_arc().await
                                                .get(&raw_connection_id)
                                                .unwrap()
                                                .lock_arc().await;

                                            let shutdown = receiver_shutdown.clone();
                                            let magic_header: Vec<u8> =
                                                QUICK_NETWORK_PACKET_HEADER.to_vec();
                                            let mut packet = Vec::<u8>::new();

                                            while
                                                let Ok(Some(data)) = receive_stream.receive().await
                                            {
                                                if
                                                    shutdown.load(Ordering::Relaxed) ||
                                                    disconnected.load(Ordering::Relaxed)
                                                {
                                                    _ = receive_stream.stop_sending((204u8).into());
                                                    tracing::info!(
                                                        "Receiving stream signaled to end."
                                                    );
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

                                                    match
                                                        QuicNetworkPacket::from_vec(
                                                            &packet_to_process
                                                        )
                                                    {
                                                        Ok(mut packet) => {
                                                            let mut client_id =
                                                                client_id.lock().await;
                                                            let author = packet.client_id.clone();

                                                            if client_id.is_none() {
                                                                *client_id = Some(author.clone());
                                                                tracing::info!(
                                                                    "{:?} Connected",
                                                                    author
                                                                );
                                                            }

                                                            let pt = packet.packet_type.clone();

                                                            if pt.eq(&PacketType::AudioFrame) {
                                                                let packet =
                                                                    update_packet_with_player_coordinates(
                                                                        packet.borrow_mut(),
                                                                        receiver_cache.clone()
                                                                    ).await;

                                                                _ = producer.push(packet);
                                                            }
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

                                            disconnected.store(true, Ordering::Relaxed);
                                            let sender = sender_collection
                                                .clone()
                                                .lock_arc().await
                                                .clone()
                                                .get(&raw_connection_id)
                                                .unwrap()
                                                .lock_arc().await;

                                            // Send an empty packet to the sending thread which'll trigger the thread to shutdown
                                            _ = sender.send(QuicNetworkPacketCollection {
                                                frames: Vec::new(),
                                                positions: PlayerDataPacket {
                                                    players: Vec::new(),
                                                },
                                            }).await;
                                            tracing::info!(
                                                "Receving stream for {} ended.",
                                                &raw_connection_id
                                            );

                                            return;
                                        })
                                    );

                                    // Sending Stream
                                    let sender_shutdown = shutdown.clone();
                                    tasks.push(
                                        tokio::spawn(async move {
                                            let disconnected = sender_disconnected.clone();
                                            let shutdown = sender_shutdown.clone();
                                            let receiver = receiver_collection
                                                .clone()
                                                .lock_arc().await
                                                .get(&raw_connection_id)
                                                .unwrap()
                                                .lock_arc().await;

                                            #[allow(irrefutable_let_patterns)]
                                            while let packet = receiver.recv().await {
                                                if shutdown.load(Ordering::Relaxed) {
                                                    tracing::info!("Sending stream was cancelled.");
                                                    // Ensure the receiving stream gets the disconnect signal
                                                    _ = disconnected.store(true, Ordering::Relaxed);
                                                    _ = send_stream.finish();
                                                    break;
                                                }

                                                if disconnected.load(Ordering::Relaxed) {
                                                    tracing::info!(
                                                        "Sending stream received shutdown signal from recv stream."
                                                    );
                                                    break;
                                                }

                                                match packet {
                                                    Ok(packet) =>
                                                        match packet.to_vec() {
                                                            Ok(rs) => {
                                                                _ = send_stream.send(
                                                                    rs.into()
                                                                ).await;
                                                                _ = send_stream.flush().await;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!(
                                                                    "{:?}",
                                                                    e.to_string()
                                                                );
                                                            }
                                                        }
                                                    Err(_) => {}
                                                }
                                            }

                                            tracing::info!(
                                                "Sending stream for {} ended.",
                                                &raw_connection_id
                                            );

                                            return;
                                        })
                                    );

                                    // Await the tasks to finish
                                    for task in tasks {
                                        _ = task.await;
                                    }

                                    // When the connection closes, remove the references in the hashmap so it doesn't grow indefinitely.
                                    tracing::info!("Connection {} closed.", &raw_connection_id);
                                    _ = outer_sender_collection
                                        .clone()
                                        .lock_arc().await
                                        .remove(&raw_connection_id);
                                    _ = outer_receiver_collection
                                        .clone()
                                        .lock_arc().await
                                        .remove(&raw_connection_id);

                                    _ = outer_producer_collection
                                        .clone()
                                        .lock_arc().await
                                        .remove(&raw_connection_id);
                                    _ = outer_consumer_collection
                                        .clone()
                                        .lock_arc().await
                                        .remove(&raw_connection_id);
                                    connection.close((99u32).into());
                                }
                                None => {
                                    tracing::info!("Bidirectional stream failed to open?");
                                }
                            }
                        Err(e) => {
                            tracing::error!(
                                "Could not accept bidirectional stream: {:?}",
                                e.to_string()
                            );
                        }
                    }
                });
            }
        })
    );

    // Iterate through the main consumer mutex and broadcast it to all active connections
    let monitor_shutdown = shutdown.clone();
    let monitor_cache = monitor_cache.clone();
    tasks.push(
        tokio::spawn(async move {
            let shutdown = monitor_shutdown.clone();
            let consumer_collection = monitor_consumer_collection.clone();
            let sender_collection = monitor_sender_collection.clone();

            loop {
                // If our monitoring thread receives the shutdown signal we need to trigger the sending streams to cancel the receiving stream, which will in turn singla the receiving stream.
                // This is a failsafe incase the stream is receiving data, but the client isn't sending data (is muted);
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("QUIC aggregation and broadcast thread signaled to terminate.");
                    let collection = QuicNetworkPacketCollection {
                        frames: Vec::new(),
                        positions: PlayerDataPacket {
                            players: Vec::new(),
                        },
                    };
                    let mut sender_collection = sender_collection.lock_arc().await.clone();
                    for (_, sender) in sender_collection.iter_mut() {
                        let sender = sender.lock_arc().await;
                        _ = sender.send(collection.clone()).await;
                    }
                    drop(sender_collection);
                    break;
                }

                let mut consumer_collection = consumer_collection.clone().lock_arc().await;

                // This is our collection of frames from all current producers (streams, clients)
                let mut frames = Vec::<QuicNetworkPacket>::new();

                // Iterate through each consumer, and get the first audio packet
                for (_, consumer) in consumer_collection.iter_mut() {
                    match consumer.pop() {
                        Ok(packet) =>
                            match packet.packet_type {
                                PacketType::AudioFrame => frames.push(packet),
                                _ => {}
                            }
                        Err(_) => {}
                    }
                }
                drop(consumer_collection);

                // Regenerate the PlayerPositionPacket from the cache
                let positions: Vec<common::Player> = monitor_cache
                    .iter()
                    .map(|(_, p)| p)
                    .collect();

                // If we don't have anything to generate, don't send it to the client
                if frames.len() != 0 {
                    // Create a QuicNetworkPacketCollection
                    // This contains all player's active position, and the most recent audio frame
                    // The client needs to interleave all the frames together to produce a final 20ms of audio to send to the output device
                    let collection = QuicNetworkPacketCollection {
                        frames,
                        positions: PlayerDataPacket { players: positions },
                    };

                    // Send this collection to every client
                    let mut sender_collection = sender_collection.lock_arc().await.clone();
                    for (_, sender) in sender_collection.iter_mut() {
                        let sender = sender.lock_arc().await;
                        _ = sender.send(collection.clone()).await;
                    }
                    drop(sender_collection);
                }

                // This is just to park the thread and release any outstanding locks
                // @todo!() We need to test this with an actual audio stream.
                _ = tokio::time::sleep(Duration::from_millis(0)).await;
            }

            return;
        })
    );

    // Provides an interface that webhook calls can push data into this QUIC server for processing.
    let t3_shutdown = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            let shutdown = t3_shutdown.clone();
            let cache = cache_c.clone();
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
                    PacketType::PlayerData =>
                        match packet.get_data() {
                            Some(data) => {
                                let data = data.to_owned();
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
                    _ => {}
                }
            }

            return;
        })
    );

    // Monitor for CTRL-C Signals to notify inbound threads that they should shutdown
    // This currently requires the theads to be active, otherwise they won't terminate
    let shutdown_monitor = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            tracing::info!("Listening for CTRL+C");
            let shutdown = shutdown_monitor.clone();
            _ = tokio::signal::ctrl_c().await;
            shutdown.store(true, Ordering::Relaxed);
            tracing::info!("Shutdown signal received, signaling QUIC threads to terminate.");

            return;
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

/// Updates an audio packet with the player coordinates at the time of rendering, if it exists
async fn update_packet_with_player_coordinates(
    packet: &mut QuicNetworkPacket,
    cache: Arc<Cache<String, common::Player>>
) -> QuicNetworkPacket {
    match packet.packet_type {
        PacketType::AudioFrame => {
            match packet.get_data() {
                Some(data) => {
                    let data = data.to_owned();
                    let data: Result<AudioFramePacket, ()> = data.try_into();

                    // If we don't have coordinates on this audio frame, add them from the cache
                    match data {
                        Ok(mut data) =>
                            match data.coordinate {
                                Some(_) => {}
                                None =>
                                    match cache.get(&data.author).await {
                                        Some(position) => {
                                            data.coordinate = Some(position.coordinates);
                                            packet.data = QuicNetworkPacketData::AudioFrame(data);
                                        }
                                        None => {}
                                    }
                            }
                        Err(_) => {
                            tracing::error!("Could not downcast_ref AudioFrame.");
                        }
                    }
                }
                None => {
                    tracing::info!("could not downcast ref audio frame.");
                }
            }
        }
        _ => {}
    }

    packet.to_owned()
}
