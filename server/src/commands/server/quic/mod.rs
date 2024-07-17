use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use common::mtlsprovider::MtlsProvider;
use common::structs::channel::ChannelEvents;
use common::structs::packet::{
    AudioFramePacket,
    ChannelEventPacket,
    PacketType,
    PlayerDataPacket,
    QuicNetworkPacket,
    QuicNetworkPacketCollection,
    QuicNetworkPacketData,
};
use moka::future::Cache;
use s2n_quic::Server;
use tokio::io::AsyncWriteExt;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use rabbitmq_stream_client::Environment;

/// Player position data
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, common::Player, RandomState>>>
> = OnceCell::new();

pub(crate) async fn get_task(
    config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>,
    _channel_cache: Arc<Mutex<Cache<String, common::structs::channel::Channel>>>
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

    // Setups mTLS for the connection
    let cert_path = format!("{}/ca.crt", &app_config.server.tls.certs_path.clone());
    let key_path = format!("{}/ca.key", &app_config.server.tls.certs_path.clone());

    let cert = Path::new(&cert_path);
    let key = Path::new(&key_path);

    let io_connection_str: &str = &format!(
        "{}:{}",
        &app_config.server.listen,
        &app_config.server.quic_port
    );

    tracing::info!("Starting QUIC server with CA: {} on {}", &cert_path, io_connection_str);

    // Initialize the server
    let mut server = Server::builder()
        .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
        .with_trusted_certificate(cert)?
        .with_certificate(cert, key)?
        .with_client_authentication()?
        .with_io(io_connection_str)?
        .start()?;

    let mut tasks = Vec::new();
    let outer_thread_shutdown = shutdown.clone();

    let rabbitmq_environment = Environment::builder()
        .host(&config.rabbitmq.host)
        .port(config.rabbitmq.port as u16)
        .build().await?;

    let producer = rabbitmq_environment
        .producer()
        .name("bvc_audioframe_producer")
        .build("audioframe_stream").await?;
    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            let app_config = app_config.clone();
            tracing::info!("Started QUIC listening server.");

            while let Some(mut connection) = server.accept().await {
                let app_config = app_config.clone();
                // Maintain the connection
                _ = connection.keep_alive(true);
                let cache = cache.clone();

                let shutdown = shutdown.clone();
                let raw_connection_id = connection.id();

                tokio::spawn(async move {
                    let cache = cache.clone();

                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    // When a client connects, store the author property on the QuicNetworkPacket for this stream
                                    // This enables us to identify packets directed to us.
                                    let cid: Option<Vec<u8>> = None;
                                    let cid_author: Option<String> = None;
                                    let recv_client_id = Arc::new(Mutex::new(cid));
                                    let send_client_id = recv_client_id.clone();

                                    let recv_cid_author = Arc::new(Mutex::new(cid_author));
                                    let send_cid_author = recv_cid_author.clone();

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
                                            let mut packet = Vec::<u8>::new();

                                            tracing::info!("Started receive stream.");

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

                                                match QuicNetworkPacket::from_stream(&mut packet) {
                                                    Ok(packets) => {
                                                        for mut p in packets {
                                                            let mut client_id =
                                                                recv_client_id.lock_arc().await;
                                                            let author = p.client_id.clone();

                                                            let mut cid_author =
                                                                recv_cid_author.lock_arc().await;
                                                            let real_author = p.author.clone();
                                                            if cid_author.is_none() {
                                                                *cid_author = Some(
                                                                    real_author.clone()
                                                                );
                                                            }

                                                            if client_id.is_none() {
                                                                *client_id = Some(author.clone());
                                                                let identifier = hex::encode(
                                                                    &author
                                                                );
                                                                tracing::info!(
                                                                    "[{}] Connected [{}] {}",
                                                                    real_author,
                                                                    identifier,
                                                                    &raw_connection_id
                                                                );
                                                            }

                                                            let pt = p.packet_type.clone();

                                                            if pt.eq(&PacketType::AudioFrame) {
                                                                let p2 =
                                                                    update_packet_with_player_coordinates(
                                                                        &mut p,
                                                                        receiver_cache.clone()
                                                                    ).await;

                                                                // Publish packet to RabbitMQ Stream
                                                                // @todo
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("{:?}", e.to_string());
                                                        continue;
                                                    }
                                                };
                                            }

                                            disconnected.store(true, Ordering::Relaxed);

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
                                            let player_channel_cache = player_channel_cache.clone();
                                            let cache = cache.clone();

                                            let mut client_id: Option<Vec<u8>> = None;
                                            let mut author: Option<String> = None;

                                            // Subscribe to Stream
                                            loop {
                                                let player_channel_cache =
                                                    player_channel_cache.clone();
                                                if receiver.is_disconnected() {
                                                    tracing::info!(
                                                        "{:?} {:?} Receiver disconnected.",
                                                        client_id,
                                                        author
                                                    );
                                                    break;
                                                }

                                                let mut packets = match receiver.recv_async().await {
                                                    Ok(packets) => packets,
                                                    Err(e) => {
                                                        tracing::info!(
                                                            "{:?} {:?} {:?}",
                                                            client_id,
                                                            author,
                                                            e
                                                        );
                                                        continue;
                                                    }
                                                };

                                                if
                                                    shutdown.load(Ordering::Relaxed) ||
                                                    disconnected.load(Ordering::Relaxed)
                                                {
                                                    tracing::info!("Sending stream was cancelled.");
                                                    drop(receiver);
                                                    // Ensure the receiving stream gets the disconnect signal
                                                    _ = disconnected.store(true, Ordering::Relaxed);
                                                    _ = send_stream.finish();
                                                    break;
                                                }

                                                if client_id.is_none() {
                                                    client_id = match
                                                        send_client_id.lock_arc().await.clone()
                                                    {
                                                        Some(item) => Some(item),
                                                        None => None,
                                                    };
                                                }

                                                if author.is_none() {
                                                    author = match
                                                        send_cid_author.lock_arc().await.clone()
                                                    {
                                                        Some(item) => Some(item),
                                                        None => None,
                                                    };
                                                }

                                                let mut packets_to_send =
                                                    QuicNetworkPacketCollection {
                                                        frames: Vec::new(),
                                                        positions: packets.positions.clone(),
                                                    };

                                                // Remove packets from the final broadcast message
                                                for packet in packets.frames.iter_mut() {
                                                    // Don't send packets back to the original broadcaster
                                                    match client_id.clone() {
                                                        Some(client_id) => {
                                                            if packet.client_id.eq(&client_id) {
                                                                continue;
                                                            }
                                                        }
                                                        None => {}
                                                    }

                                                    let is_in_group_or_in_range: bool =
                                                        can_hear_audio(
                                                            packet,
                                                            author.clone(),
                                                            player_channel_cache.clone(),
                                                            app_config.clone()
                                                        ).await;

                                                    if is_in_group_or_in_range {
                                                        packets_to_send.frames.push(packet.clone());
                                                    }
                                                }

                                                match packets_to_send.to_vec() {
                                                    Ok(rs) => {
                                                        _ = send_stream.write_all(&rs).await;
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

    // Provides an interface that webhook calls can push data into this QUIC server for processing.
    let t3_shutdown = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            let player_channel_cache = monitor_player_channel_cache.clone();
            let shutdown = t3_shutdown.clone();
            let cache = cache_c.clone();
            let queue = queue.clone();

            #[allow(irrefutable_let_patterns)]
            while let packet = queue.pop().await {
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Webhook QUIC thread ended.");
                    return;
                }

                let pkc = packet.clone();

                // Packet specific handling
                match pkc.packet_type {
                    // Add the player position packet to the cache
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
                    // Store the player channel if one is set
                    PacketType::ChannelEvent => {
                        match packet.get_data() {
                            Some(data) => {
                                let data = data.to_owned();
                                let data: Result<ChannelEventPacket, ()> = data.try_into();

                                match data {
                                    Ok(data) => {
                                        tracing::info!(
                                            "[{}] {:?} {}",
                                            data.name,
                                            data.event,
                                            data.channel
                                        );
                                        match data.event {
                                            ChannelEvents::Join => {
                                                _ = player_channel_cache
                                                    .lock_arc().await
                                                    .insert(data.name, data.channel).await;
                                            }
                                            ChannelEvents::Leave => {
                                                _ = player_channel_cache
                                                    .lock_arc().await
                                                    .remove(&data.name).await;
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            None => {}
                        };
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

/// Returns true if the player in the author frame should be able to hear the packet
/// This is based on if the player is in the same channel as the user or if they are within broadcast range
/// This is done to cut down on the server => client bandwidth
async fn can_hear_audio(
    packet: &mut QuicNetworkPacket,
    author: Option<String>,
    player_channel_cache: Arc<Mutex<Cache<String, String>>>,
    app_config: ApplicationConfig
) -> bool {
    let cache = match get_cache().await {
        Ok(cache) => cache,
        Err(_) => {
            tracing::error!(
                "A cache was constructed but didn't survive. This shouldn't happen. Restart bvc server."
            );
            return false;
        }
    };

    match packet.clone().get_data() {
        Some(data) => {
            match data.to_owned().try_into() {
                Ok(data) => {
                    let data: AudioFramePacket = data;
                    match author.clone() {
                        Some(author) => {
                            // Add the packet if the player is in the same group
                            let player_channels = player_channel_cache.lock_arc().await.clone();
                            let this_player = player_channels.get(&author).await;
                            let packet_author = player_channels.get(&data.author).await;

                            // If the two players are in the same channel
                            // Individual user matching is done by the client_id, which is unique per connection
                            // The server filters out identical connections
                            // But a player may receive their own audio from a different source if they are the emitter
                            match
                                this_player.is_some() &&
                                packet_author.is_some() &&
                                this_player.eq(&packet_author)
                            {
                                true => {
                                    packet.in_group = Some(true);
                                    true
                                }

                                // Add the packet if the player is within the server defined audio range
                                false =>
                                    match cache.get(&author).await {
                                        Some(position) => {
                                            let c1 = position.coordinates;

                                            let data_author_dimension = cache.get(
                                                &data.author
                                            ).await;

                                            match data.coordinate {
                                                Some(c2) => {
                                                    // Calcuate 3d spatial distance based upon the configured broadcast range
                                                    let distance = (
                                                        (c1.x - c2.x).powf(2.0) +
                                                        (c1.y - c2.y).powf(2.0) +
                                                        (c1.z - c2.z).powf(2.0)
                                                    ).sqrt();

                                                    // Distance calculation only applies within the same dimension, and based upon the player matched coordinates.
                                                    if
                                                        data_author_dimension.is_some() &&
                                                        data_author_dimension
                                                            .unwrap()
                                                            .dimension.eq(&position.dimension) &&
                                                        distance <=
                                                            (3.0_f32).sqrt() *
                                                                app_config.voice.broadcast_range
                                                    {
                                                        true
                                                    } else {
                                                        false
                                                    }
                                                }
                                                None => { false }
                                            }
                                        }
                                        None => false,
                                    }
                            }
                        }
                        None => false,
                    }
                }
                Err(_) => false,
            }
        }
        None => false,
    }
}
