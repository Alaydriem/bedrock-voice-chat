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

use async_channel::{ unbounded, Receiver, Sender };
const QUEUE_CAPACITY: usize = 100000;

#[derive(Debug, Clone, Eq, PartialEq)]
enum MessageEventType {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
enum MessageEventData {
    Sender(Sender<QuicNetworkPacketCollection>),
    Receiver(Receiver<QuicNetworkPacket>),
}

#[derive(Debug, Clone)]
struct MessageEvent {
    pub connection_id: u64,
    pub event_type: MessageEventType,
    pub thing: Option<MessageEventData>,
}

/// Player position data
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, common::Player, RandomState>>>
> = OnceCell::new();

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

    let mut tasks = Vec::new();
    let outer_thread_shutdown = shutdown.clone();
    let (broadcast, recv_broadcast) = unbounded::<MessageEvent>();

    let broadcast: Arc<_> = Arc::new(broadcast);
    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let broadcast = broadcast.clone();
            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            tracing::info!("Started QUIC listening server.");

            while let Some(mut connection) = server.accept().await {
                let broadcast = broadcast.clone();
                // Maintain the connection
                _ = connection.keep_alive(true);
                let cache = cache.clone();

                let shutdown = shutdown.clone();
                let raw_connection_id = connection.id();

                let (producer, consumer) = unbounded::<QuicNetworkPacket>();
                let (sender, receiver) = unbounded::<QuicNetworkPacketCollection>();

                // send consumer and receiver to monitor thread
                _ = broadcast.try_send(MessageEvent {
                    connection_id: raw_connection_id,
                    event_type: MessageEventType::Add,
                    thing: Some(MessageEventData::Sender(sender)),
                });

                _ = broadcast.try_send(MessageEvent {
                    connection_id: raw_connection_id,
                    event_type: MessageEventType::Add,
                    thing: Some(MessageEventData::Receiver(consumer)),
                });

                let producer = Arc::new(producer);
                let receiver = Arc::new(receiver);

                tokio::spawn(async move {
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

                                    let cancel_broadcast = broadcast.clone();

                                    // Receiving Stream
                                    tasks.push(
                                        tokio::spawn(async move {
                                            let disconnected = receiver_disconnected.clone();

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
                                                                    "{:?} Connected {}",
                                                                    author,
                                                                    &raw_connection_id
                                                                );
                                                            }

                                                            let pt = packet.packet_type.clone();

                                                            if pt.eq(&PacketType::AudioFrame) {
                                                                let packet =
                                                                    update_packet_with_player_coordinates(
                                                                        packet.borrow_mut(),
                                                                        receiver_cache.clone()
                                                                    ).await;

                                                                _ = producer.send(packet).await;
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

                                            // Send notice to other threads to stop using them
                                            _ = broadcast.try_send(MessageEvent {
                                                connection_id: raw_connection_id,
                                                event_type: MessageEventType::Remove,
                                                thing: None,
                                            });

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

                                            #[allow(irrefutable_let_patterns)]
                                            while let Ok(packet) = receiver.recv().await {
                                                if
                                                    shutdown.load(Ordering::Relaxed) ||
                                                    disconnected.load(Ordering::Relaxed)
                                                {
                                                    tracing::info!("Sending stream was cancelled.");
                                                    // Ensure the receiving stream gets the disconnect signal
                                                    _ = disconnected.store(true, Ordering::Relaxed);
                                                    _ = send_stream.finish();
                                                    break;
                                                }

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
                                    _ = cancel_broadcast.try_send(MessageEvent {
                                        connection_id: raw_connection_id,
                                        event_type: MessageEventType::Remove,
                                        thing: None,
                                    });

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

            let mut consumers: HashMap<u64, Receiver<QuicNetworkPacket>> = HashMap::new();
            let mut senders: HashMap<u64, Sender<QuicNetworkPacketCollection>> = HashMap::new();

            loop {
                match recv_broadcast.try_recv() {
                    Ok(event) => {
                        let event: MessageEvent = event;
                        match event.event_type {
                            MessageEventType::Add => {
                                let thing = event.thing.unwrap();
                                match thing {
                                    MessageEventData::Sender(sender) => {
                                        senders.insert(event.connection_id, sender);
                                    }
                                    MessageEventData::Receiver(consumer) => {
                                        consumers.insert(event.connection_id, consumer);
                                    }
                                }
                            }
                            MessageEventType::Remove => {
                                consumers.remove(&event.connection_id);
                                senders.remove(&event.connection_id);
                            }
                        }
                    }
                    Err(_) => {}
                }

                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("QUIC aggregation and broadcast thread signaled to terminate.");
                    let collection = QuicNetworkPacketCollection {
                        frames: Vec::new(),
                        positions: PlayerDataPacket {
                            players: Vec::new(),
                        },
                    };

                    // Send a empty packet to all receiving streams so they process the shutdown signal incase there isn't incoming data
                    // Then remove the item from the collections, and drop the sender
                    for (id, sender) in senders.iter() {
                        if sender.send(collection.clone()).await.is_err() {
                            sender.close();
                        }
                    }
                    break;
                }

                // This is our collection of frames from all current producers (streams, clients)
                let mut frames = Vec::<QuicNetworkPacket>::new();

                for (id, consumer) in consumers.iter() {
                    if consumer.is_closed() {
                        continue;
                    }
                    // 1. Awaiting the stream awaits this thread,
                    //  // If there's no data in the buffer
                    //  // Or if the client disconnected but it hasn't been detected yet.
                    // 2. as_sync().recv() doesn't parallelize
                    // I need a spsc that await doesn't hold, it just returns immediately
                    // Is QUIC simply too slow?
                    let packet = consumer.recv().await;
                    if packet.is_err() {
                        consumer.close();
                    }
                    match packet {
                        Ok(frame) => {
                            match frame.packet_type {
                                PacketType::AudioFrame => frames.push(frame),
                                _ => {}
                            }
                        }
                        Err(_) => {}
                    }
                }

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
                    for (id, sender) in senders.iter() {
                        if sender.send(collection.clone()).await.is_err() {
                            sender.close();
                        }
                    }
                }
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
