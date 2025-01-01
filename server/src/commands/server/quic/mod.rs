use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use common::ncryptflib::rocket::Utc;
use common::structs::channel::ChannelEvents;
use common::structs::packet::{
    ChannelEventPacket,
    PacketOwner,
    PacketType,
    PlayerDataPacket,
    QuicNetworkPacket,
};
use moka::future::Cache;
use rabbitmq_stream_client::error::StreamCreateError;
use rabbitmq_stream_client::types::{ Message, OffsetSpecification, ResponseCode };
use rocket::futures::StreamExt;
use s2n_quic::Server;
use tokio::io::AsyncWriteExt;
use std::collections::hash_map::RandomState;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use rabbitmq_stream_client::Environment;

pub const AUDIO_STREAM_NAME: &str = "audio_stream";

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

    let player_channel_cache = Arc::new(
        async_mutex::Mutex::new(
            moka::future::Cache::<String, String>::builder().max_capacity(100).build()
        )
    );

    let monitor_player_channel_cache = player_channel_cache.clone();

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

    // Setups mTLS for the connection
    let cert_path = format!("{}/ca.crt", &app_config.server.tls.certs_path.clone());
    let key_path = format!("{}/ca.key", &app_config.server.tls.certs_path.clone());

    let io_connection_str: &str = &format!(
        "{}:{}",
        &app_config.server.listen,
        &app_config.server.quic_port
    );

    tracing::info!("Starting QUIC server with CA: {} on {}", &cert_path, io_connection_str);

    let provider = common::rustls::MtlsProvider::new(&cert_path, &cert_path, &key_path).await?;

    // Initialize the server
    let mut server = Server::builder()
        .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
        .with_tls(provider)?
        .with_io(io_connection_str)?
        .start()?;

    let mut tasks = Vec::new();
    let outer_thread_shutdown = shutdown.clone();

    let rabbitmq_environment_raw = match
        Environment::builder()
            .heartbeat(5)
            .host(&config.rabbitmq.host)
            .port(config.rabbitmq.port as u16)
            .build().await
    {
        Ok(environment) => environment,
        Err(e) => {
            tracing::error!("Failed to connect to RabbitMQ Stream: {}", e.to_string());
            panic!("{:?}", e);
        }
    };
    let rabbitmq_environment = Arc::new(rabbitmq_environment_raw);

    let create_response = rabbitmq_environment
        .stream_creator()
        .max_age(Duration::from_secs(60))
        .create(AUDIO_STREAM_NAME).await;

    if let Err(e) = create_response {
        if let StreamCreateError::Create { stream, status } = e {
            match status {
                // we can ignore this error because the stream already exists
                ResponseCode::StreamAlreadyExists => {}
                err => {
                    tracing::error!("Error creating stream: {:?} {:?}", stream, err);
                }
            }
        }
    }

    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let rabbitmq_environment = rabbitmq_environment.clone();
            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            let app_config = app_config.clone();
            let player_channel_cache = player_channel_cache.clone();
            tracing::info!("Started QUIC listening server.");

            while let Some(mut connection) = server.accept().await {
                let rabbitmq_environment = rabbitmq_environment.clone();

                let player_channel_cache = player_channel_cache.clone();
                let app_config = app_config.clone();
                // Maintain the connection
                _ = connection.keep_alive(true);
                let cache = cache.clone();

                let shutdown = shutdown.clone();
                let raw_connection_id = connection.id();

                tokio::spawn(async move {
                    let cache = cache.clone();
                    let player_channel_cache = player_channel_cache.clone();
                    let stream_cache = cache.clone();

                    let rabbitmq_environment = rabbitmq_environment.clone();
                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    let rabbitmq_recv_environment = rabbitmq_environment.clone();
                                    let rabbitmq_send_environment = rabbitmq_environment.clone();
                                    let player_channel_cache = player_channel_cache.clone();
                                    let receiver_shutdown = shutdown.clone();
                                    let receiver_cache = stream_cache.clone();

                                    let whoami: Option<PacketOwner> = None;
                                    let recv_whoami = Arc::new(Mutex::new(whoami));
                                    let send_whoami = recv_whoami.clone();

                                    let mut tasks = Vec::new();

                                    let disconnected = Arc::new(AtomicBool::new(false));
                                    let receiver_disconnected = disconnected.clone();
                                    let sender_disconnected = disconnected.clone();

                                    // Receiving Stream
                                    tasks.push(
                                        tokio::spawn(async move {
                                            let producer = match
                                                rabbitmq_recv_environment
                                                    .clone()
                                                    .producer()
                                                    .build(AUDIO_STREAM_NAME).await
                                            {
                                                Ok(producer) => producer,
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Producer Create Error: {:?}",
                                                        e
                                                    );
                                                    return;
                                                }
                                            };
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
                                                        for mut raw_network_packet in packets {
                                                            let mut owner =
                                                                recv_whoami.lock_arc().await;
                                                            if owner.is_none() {
                                                                *owner = Some(
                                                                    raw_network_packet.owner.clone()
                                                                );
                                                            }

                                                            match
                                                                raw_network_packet.get_packet_type()
                                                            {
                                                                PacketType::AudioFrame => {
                                                                    // Update the packets with player coordinate data
                                                                    raw_network_packet.update_coordinates(
                                                                        receiver_cache.clone()
                                                                    ).await;

                                                                    // Return back to string
                                                                    match
                                                                        raw_network_packet.to_string()
                                                                    {
                                                                        Ok(message) => {
                                                                            // Push message to consumers
                                                                            _ =
                                                                                producer.send_with_confirm(
                                                                                    Message::builder()
                                                                                        .body(
                                                                                            message
                                                                                        )
                                                                                        .build()
                                                                                ).await;
                                                                        }
                                                                        Err(e) => {
                                                                            tracing::error!(
                                                                                "Send Error: {}",
                                                                                e.to_string()
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "{:?} {}",
                                                            e,
                                                            e.to_string()
                                                        );
                                                        continue;
                                                    }
                                                };
                                            }

                                            match producer.close().await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    tracing::error!(
                                                        "ProducerCloseError: {}",
                                                        e.to_string()
                                                    );
                                                }
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
                                            let cache = cache.clone();

                                            let mut owner: Option<PacketOwner> = None;

                                            let dt = Utc::now();
                                            let now: i64 = dt.timestamp();
                                            let mut consumer = match
                                                rabbitmq_send_environment
                                                    .clone()
                                                    .consumer()
                                                    .offset(OffsetSpecification::Timestamp(now))
                                                    .build(AUDIO_STREAM_NAME).await
                                            {
                                                Ok(consumer) => consumer,
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Recieving Stream: {}",
                                                        e.to_string()
                                                    );
                                                    return;
                                                }
                                            };

                                            while let Some(delivery) = consumer.next().await {
                                                let quic_network_packet = match delivery {
                                                    Ok(delivery) =>
                                                        delivery
                                                            .message()
                                                            .data()
                                                            .map(|data|
                                                                ron
                                                                    ::from_str::<QuicNetworkPacket>(
                                                                        &String::from_utf8_lossy(
                                                                            data
                                                                        )
                                                                    )
                                                                    .unwrap()
                                                            )
                                                            .unwrap(),
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "Consumer Delivery Error: {}",
                                                            e.to_string()
                                                        );
                                                        continue;
                                                    }
                                                };

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

                                                // Determine the owner
                                                if owner.is_none() {
                                                    owner = match
                                                        send_whoami.lock_arc().await.clone()
                                                    {
                                                        Some(owner) => Some(owner),
                                                        None => None,
                                                    };
                                                }

                                                // If the owner is still not set, we can't calculate the data, give up
                                                if owner.is_none() {
                                                    continue;
                                                }

                                                if
                                                    quic_network_packet.is_receivable(
                                                        owner.clone().unwrap(),
                                                        player_channel_cache.clone(),
                                                        cache.clone(),
                                                        app_config.clone().voice.broadcast_range
                                                    ).await
                                                {
                                                    match quic_network_packet.to_vec() {
                                                        Ok(rs) => {
                                                            _ = send_stream.write_all(&rs).await;
                                                        }
                                                        Err(e) => {
                                                            tracing::error!(
                                                                "Send Stream Write: {:?}",
                                                                e.to_string()
                                                            );
                                                        }
                                                    };
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
    let monitor_player_channel_cache = monitor_player_channel_cache.clone();
    tasks.push(
        tokio::spawn(async move {
            let shutdown = t3_shutdown.clone();
            let queue = queue.clone();
            let cache = cache_c.clone();

            let player_channel_cache = monitor_player_channel_cache.clone();

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
