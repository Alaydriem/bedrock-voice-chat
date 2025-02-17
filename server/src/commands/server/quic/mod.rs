use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_mutex::Mutex;
use async_once_cell::OnceCell;
use common::ncryptflib::rocket::Utc;
use common::structs::channel::ChannelEvents;
use common::structs::packet::{
    ChannelEventPacket, DebugPacket, PacketOwner, PacketType, PlayerDataPacket, QuicNetworkPacket,
    QuicNetworkPacketData,
};
use moka::future::Cache;
use s2n_quic::Server;
use std::collections::hash_map::RandomState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;
use zeromq::{Socket, SocketRecv, SocketSend};

type MessageQueue = deadqueue::limited::Queue<QuicNetworkPacket>;

/// Player position data
pub(crate) static PLAYER_POSITION_CACHE: OnceCell<
    Option<Arc<Cache<String, common::Player, RandomState>>>,
> = OnceCell::new();

pub(crate) async fn get_task(
    config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>,
    _channel_cache: Arc<Mutex<Cache<String, common::structs::channel::Channel>>>,
) -> Result<Vec<JoinHandle<()>>, anyhow::Error> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_config = config.clone();
    let app_config = config.clone();

    let player_channel_cache = Arc::new(async_mutex::Mutex::new(
        moka::future::Cache::<String, String>::builder()
            .max_capacity(100)
            .build(),
    ));

    let message_queue = Arc::new(MessageQueue::new(5000));
    let recv_message_queue = message_queue.clone();
    let shutdown_message_queue = message_queue.clone();

    let monitor_player_channel_cache = player_channel_cache.clone();

    // Instantiate the player position cache
    // Player positions are valid for 5 minutes before they are evicted
    PLAYER_POSITION_CACHE
        .get_or_init(async {
            return Some(Arc::new(
                moka::future::Cache::builder()
                    .time_to_live(Duration::from_secs(300))
                    .max_capacity(256)
                    .build(),
            ));
        })
        .await;

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
        &app_config.server.listen, &app_config.server.quic_port
    );

    tracing::info!(
        "Starting QUIC server with CA: {} on {}",
        &cert_path,
        io_connection_str
    );

    let provider = common::rustls::MtlsProvider::new(&cert_path, &cert_path, &key_path).await?;

    // Initialize the server
    let mut server = Server::builder()
        .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
        .with_tls(provider)?
        .with_io(io_connection_str)?
        .start()?;

    let mut tasks = Vec::new();
    let outer_thread_shutdown = shutdown.clone();

    let broadcast_range = app_config.voice.broadcast_range;

    // This is our main thread for the QUIC server
    tasks.push(
        tokio::spawn(async move {
            let message_queue = message_queue.clone();
            let shutdown = outer_thread_shutdown.clone();
            let cache = cache.clone();
            let player_channel_cache = player_channel_cache.clone();
            tracing::info!("Started QUIC listening server.");

            while let Some(mut connection) = server.accept().await {
                if shutdown.load(Ordering::Relaxed) {
                    tracing::info!("Webhook Connection Receiving Thread Ended");
                    return;
                }

                let message_queue = message_queue.clone();
                let player_channel_cache = player_channel_cache.clone();
                _ = connection.keep_alive(true);
                let cache = cache.clone();

                let shutdown = shutdown.clone();
                let raw_connection_id = connection.id();

                tokio::spawn(async move {
                    let message_queue = message_queue.clone();
                    let cache = cache.clone();
                    let player_channel_cache = player_channel_cache.clone();
                    let stream_cache = cache.clone();
                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    let receiver_shutdown = shutdown.clone();
                                    let receiver_cache = stream_cache.clone();
                                    let sender_cache = stream_cache.clone();

                                    let whoami: Option<PacketOwner> = None;
                                    let recv_whoami = Arc::new(Mutex::new(whoami));
                                    let send_whoami = recv_whoami.clone();

                                    let mut tasks = Vec::new();

                                    let disconnected = Arc::new(AtomicBool::new(false));
                                    let receiver_disconnected = disconnected.clone();
                                    let sender_disconnected = disconnected.clone();

                                    let message_queue = message_queue.clone();

                                    // Receiving Stream
                                    tasks.push(
                                        tokio::spawn(async move {
                                            let message_queue = message_queue.clone();
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
                                                            // Determine the packet owner from the message
                                                            let mut owner =
                                                                recv_whoami.lock_arc().await;
                                                            if owner.is_none() {
                                                                tracing::info!(
                                                                    "{} Connected to recv stream.",
                                                                    raw_network_packet.get_author()
                                                                );
                                                                *owner = Some(
                                                                    raw_network_packet.owner.clone().unwrap()
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

                                                                    _ =
                                                                        message_queue.push(
                                                                            raw_network_packet
                                                                        ).await;
                                                                }
                                                                _ => {}
                                                            };
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

                                            let mut owner: Option<PacketOwner> = None;

                                            let dt = Utc::now();
                                            let now: i64 = dt.timestamp();

                                            let mut owner_wait_time_counter = 0;
                                            // Determine the owner before starting the consumer
                                            while owner.is_none() {
                                                owner = match send_whoami.lock_arc().await.clone() {
                                                    Some(owner) => {
                                                        let dt2 = Utc::now();
                                                        let n2: i64 = dt2.timestamp();
                                                        tracing::info!(
                                                            "{} connected to sending stream after {} attempts {}",
                                                            owner.name,
                                                            owner_wait_time_counter,
                                                            n2 - now
                                                        );
                                                        Some(owner)
                                                    }
                                                    None => None,
                                                };

                                                // Wait before trying to get the owner again
                                                if owner.is_none() {
                                                    owner_wait_time_counter =
                                                        owner_wait_time_counter + 1;
                                                    _ = tokio::time::sleep(
                                                        Duration::from_micros(5)
                                                    );
                                                }
                                            }

                                            let mut zeromq_socket = zeromq::SubSocket::new();
                                            _ = zeromq_socket.connect("tcp://127.0.0.1:5556").await;
                                            _ = zeromq_socket.subscribe("").await;

                                            // Consume messags iteratively in a while loop
                                            while let Ok(zmq_message) = zeromq_socket.recv().await {
                                                let message: String = match
                                                    String::from_utf8(
                                                        zmq_message.get(0).unwrap().to_vec()
                                                    )
                                                {
                                                    Ok(message) => message,
                                                    Err(e) => {
                                                        tracing::error!("{:?}", e);
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

                                                match QuicNetworkPacket::from_string(message) {
                                                    Some(packet) => {
                                                        // Determine if the reciever can even receive the packet
                                                        if
                                                            !packet.is_receivable(
                                                                owner.clone().unwrap(),
                                                                player_channel_cache.clone(),
                                                                sender_cache.clone(),
                                                                broadcast_range
                                                            ).await
                                                        {
                                                            continue;
                                                        }

                                                        match packet.to_vec() {
                                                            Ok(rs) => {
                                                                _ = send_stream.write_all(
                                                                    &rs
                                                                ).await;
                                                            }
                                                            Err(e) => {
                                                                tracing::error!(
                                                                    "Send Stream Write: {:?}",
                                                                    e.to_string()
                                                                );
                                                            }
                                                        };
                                                    }
                                                    None => {}
                                                }
                                            }

                                            _ = zeromq_socket.close().await;
                                            tracing::info!(
                                                "Sending stream for {} [{}] ended.",
                                                owner.clone().unwrap().name,
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

    let t4_shutdown = shutdown.clone();
    // Shared ZeroMQ publisher thread
    tasks.push(tokio::spawn(async move {
        let shutdown = t4_shutdown.clone();
        let message_queue = recv_message_queue.clone();
        let mut socket = zeromq::PubSocket::new();
        match socket.bind("tcp://127.0.0.1:5556").await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("{:?}", e);
            }
        }

        #[allow(irrefutable_let_patterns)]
        while let packet = message_queue.pop().await {
            if shutdown.load(Ordering::Relaxed) {
                tracing::info!("Relay QUIC thread ended.");
                break;
            }

            match packet.to_string() {
                Ok(message) => match socket.send(message.into()).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("failed to send message {:?}", e);
                    }
                },
                Err(e) => {
                    tracing::error!("Couldn't convert message {:?}", e);
                }
            }
        }

        _ = socket.close().await;
    }));

    // Provides an interface that webhook calls can push data into this QUIC server for processing.
    let t3_shutdown = shutdown.clone();
    let monitor_player_channel_cache = monitor_player_channel_cache.clone();
    let shutdown_queue = queue.clone();

    tasks.push(tokio::spawn(async move {
        let shutdown = t3_shutdown.clone();
        let queue = queue.clone();
        let cache = cache_c.clone();

        let player_channel_cache = monitor_player_channel_cache.clone();

        let mut socket = zeromq::PubSocket::new();
        match socket.bind("tcp://127.0.0.1:5556").await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("{:?}", e);
            }
        }

        #[allow(irrefutable_let_patterns)]
        while let packet = queue.pop().await {
            if shutdown.load(Ordering::Relaxed) {
                tracing::info!("Webhook QUIC thread ended.");
                return;
            }

            let pkc = packet.clone();

            match pkc.to_string() {
                Ok(message) =>
                // Push the player position data to all active players.
                // This provides players with both positional information,
                // And acts as a pulse-clock to force events to be processed
                // on the clients
                {
                    match socket.send(message.into()).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("failed to send message {:?}", e);
                        }
                    }
                }

                Err(e) => {
                    tracing::error!("Couldn't convert message {:?}", e);
                }
            };

            // Packet specific handling
            match pkc.packet_type {
                // Add the player position packet to the cache
                PacketType::PlayerData => match packet.get_data() {
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
                },
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
                                                .lock_arc()
                                                .await
                                                .insert(data.name, data.channel)
                                                .await;
                                        }
                                        ChannelEvents::Leave => {
                                            _ = player_channel_cache
                                                .lock_arc()
                                                .await
                                                .remove(&data.name)
                                                .await;
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
    }));

    // Monitor for CTRL-C Signals to notify inbound threads that they should shutdown
    // This currently requires the theads to be active, otherwise they won't terminate
    let shutdown_monitor = shutdown.clone();
    tasks.push(
        tokio::spawn(async move {
            tracing::info!("Listening for CTRL+C");
            let shutdown = shutdown_monitor.clone();
            _ = tokio::signal::ctrl_c().await;
            shutdown.store(true, Ordering::Relaxed);

            let shutdown_packet = QuicNetworkPacket {
                packet_type: PacketType::Debug,
                owner: Some(PacketOwner {
                    name: "shutdown".into(),
                    client_id: (0..32).map(|_| rand::random::<u8>()).collect()
                }),
                data: QuicNetworkPacketData::Debug(DebugPacket("Shutdown signal received.".into()))
            };

            // Push a message into the channel cache
            shutdown_queue.push(shutdown_packet.clone()).await;

            // Push a message into the message queue
            shutdown_message_queue.push(shutdown_packet.clone()).await;

            tracing::info!("Shutdown signal received, signaling QUIC threads to terminate. Waiting 3 seconds before force shutdown.");

            tokio::time::sleep(Duration::from_secs(3)).await;

            // Shutdown the QUIC listener thread by spawning a fake connection then immediately dropping it
            _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs
                ::default_provider()
                .install_default();
            let certs_path = shutdown_config.server.tls.certs_path.clone();
            let ca = format!("{}/ca.crt", certs_path);
            let key = format!("{}/ca.key", certs_path);
            let p = common::rustls::MtlsProvider::new(
                std::path::Path::new(&ca),
                std::path::Path::new(&ca),
                std::path::Path::new(&key)
            ).await.unwrap();

            let addr: std::net::SocketAddr = format!("127.0.0.1:{}", shutdown_config.server.quic_port).parse().unwrap();
            let connect = s2n_quic::client::Connect::new(addr).with_server_name("localhost");
            let client = s2n_quic::Client::builder().with_tls(p)
                .unwrap()
                .with_io("0.0.0.0:0")
                .unwrap()
                .start()
                .unwrap();

            client.connect(connect)
                .await
                .unwrap()
                .close(s2n_quic::application::Error::UNKNOWN.into());

            tracing::info!("All quic threads are now forcefully shutting down.");
            return;
        })
    );

    return Ok(tasks);
}

/// Returns the cache object without if match branching nonsense
async fn get_cache() -> Result<Arc<Cache<String, common::Player>>, anyhow::Error> {
    match PLAYER_POSITION_CACHE.get() {
        Some(cache) => match cache {
            Some(cache) => Ok(cache.clone()),
            None => Err(anyhow!("Cache not found.")),
        },

        None => Err(anyhow!("Cache not found")),
    }
}
