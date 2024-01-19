use common::{
    mtlsprovider::MtlsProvider,
    structs::packet::{
        AudioFramePacket,
        DebugPacket,
        PacketType,
        QuicNetworkPacket,
        QUICK_NETWORK_PACKET_HEADER,
    },
};

use std::{ collections::HashMap, sync::Arc };

use anyhow::anyhow;
use async_once_cell::OnceCell;
use moka::future::Cache;
use rand::distributions::{ Alphanumeric, DistString };
use s2n_quic::{ client::Connect, stream::{ ReceiveStream, SendStream }, Client };
use std::net::SocketAddr;
use tauri::State;

pub(crate) static NETWORK_STATE_CACHE: OnceCell<
    Option<Arc<Cache<String, String, std::collections::hash_map::RandomState>>>
> = OnceCell::new();

const SENDER: &str = "send_stream";
//const RECEIVER: &str = "receive_stream";

#[tauri::command(async)]
pub(crate) async fn network_stream(
    audio_producer: State<'_, Arc<kanal::Sender<AudioFramePacket>>>,
    rx: State<'_, Arc<kanal::Receiver<QuicNetworkPacket>>>
) -> Result<bool, bool> {
    // Stop any existing streams
    stop_network_stream().await;

    let rx = rx.inner().clone();

    // Create a new job for the thread worker to execute in
    let (id, cache) = match setup_task_cache(SENDER).await {
        Ok((id, cache)) => (id, cache),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    };

    let provider = match get_mtls_provider().await {
        Ok(provider) => provider,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            stop_network_stream().await;
            return Err(false);
        }
    };

    let client = match get_quic_client(provider).await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            stop_network_stream().await;
            return Err(false);
        }
    };

    let (mut send_stream, mut receive_stream) = match get_stream(client).await {
        Ok((s, r)) => (s, r),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            stop_network_stream().await;
            return Err(false);
        }
    };

    // Sending Stream
    let send_id = id.clone();

    let gamertag = match super::credentials::get_credential("gamertag".into()).await {
        Ok(gt) => gt,
        Err(_) => {
            return Err(false);
        }
    };

    // Self assigned consistent packet_id to identify this stream to the quic server
    let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    tokio::spawn(async move {
        let rx = rx.clone();
        let id = send_id.clone();

        let packet = QuicNetworkPacket {
            client_id: client_id.clone(),
            packet_type: common::structs::packet::PacketType::Debug,
            author: gamertag.clone(),
            data: common::structs::packet::QuicNetworkPacketData::Debug(
                DebugPacket(gamertag.clone())
            ),
        };

        match packet.to_vec() {
            Ok(reader) => {
                _ = send_stream.send(reader.into()).await;
                _ = send_stream.flush().await;
            }
            Err(e) => {
                tracing::error!("{}", e.to_string());
            }
        }

        #[allow(irrefutable_let_patterns)]
        while let packet = rx.recv() {
            match packet {
                Ok(mut packet) => {
                    if super::should_self_terminate(&id, &cache, SENDER).await {
                        break;
                    }

                    packet.client_id = client_id.clone();
                    packet.author = gamertag.clone();
                    match packet.to_vec() {
                        Ok(reader) => {
                            _ = send_stream.send(reader.into()).await;
                            _ = send_stream.flush().await;
                        }
                        Err(e) => {
                            tracing::info!("{}", e.to_string());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("QuicNetworkPacket was not valid? {}", e.to_string());
                }
            }
        }
    });

    // Recv stream
    let recv_id = id.clone();
    let audio_producer = audio_producer.inner().clone();
    tokio::spawn(async move {
        let cache = cache.clone();
        let id = recv_id.clone();

        let magic_header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
        let mut packet = Vec::<u8>::new();
        while let Ok(Some(data)) = receive_stream.receive().await {
            if super::should_self_terminate(&id, &cache, SENDER).await {
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

            let packet_len = usize::from_be_bytes(packet_length.unwrap().try_into().unwrap());

            // If the current packet starts with the magic header and we have enough bytes, drain it
            if packet_header.eq(&magic_header) && packet.len() >= packet_len + 13 {
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
                let packet_to_process = packet_to_process.get(13..packet_to_process.len()).unwrap();

                match QuicNetworkPacket::from_vec(&packet_to_process) {
                    Ok(packet) => {
                        match packet.packet_type {
                            // Audio frames should be pushed into the audio_producer mux
                            // To be handled by the output stream
                            PacketType::AudioFrame => {
                                match packet.get_data() {
                                    Some(data) => {
                                        let data = data.to_owned();
                                        let data: Result<AudioFramePacket, ()> = data.try_into();

                                        // If we don't have coordinates on this audio frame, add them from the cache
                                        match data {
                                            Ok(data) => {
                                                let audio_producer = audio_producer.clone();
                                                _ = audio_producer.send(data);
                                            }
                                            Err(_) => {
                                                tracing::error!(
                                                    "Could not decode AudioFramePacket"
                                                );
                                            }
                                        }
                                    }
                                    None => {
                                        tracing::error!("Packet didn't have the right data?");
                                    }
                                };
                            }
                            PacketType::PlayerData => {}
                            PacketType::Debug => {}
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Unable to deserialize RON packet. Possible packet length issue? {}",
                            e.to_string()
                        );
                        continue;
                    }
                }
            }
        }
    });
    Ok(true)
}

/// Returns true if the network stream is active by measurement of a cache key being present
#[tauri::command(async)]
pub(crate) async fn is_network_stream_active() -> bool {
    let cache_key = SENDER;
    match NETWORK_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) =>
                    match cache.get(cache_key).await {
                        Some(_) => {
                            return true;
                        }
                        None => {
                            return false;
                        }
                    }
                None => {
                    return false;
                }
            }
        None => {
            return false;
        }
    }
}

/// Stops the network stream
/// This works by clearing the SENDER cache key
/// And the send/recv loops check to see if this is still present
/// If the key is removed, the streams are signaled to shutdown
/// There may be a delay in the shutdown of the stream
#[tauri::command(async)]
pub(crate) async fn stop_network_stream() -> bool {
    let cache_key = SENDER;

    match NETWORK_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
                    return true;
                }
                None => {
                    return false;
                }
            }
        None => {
            return false;
        }
    }
}

/// Sets up the task cache with the correct values
/// We're storing the current job inside of the cache as a single value
/// When this task launches, we replace the entire cache key with single element containing only this id
/// We're using a hashmap to make a single lookup with a HashMap::<String, id>::new() value
/// Where the String is the self identifier of _this_ thread, and the id is the job running status
/// When this thread launches, we consider all other threads invalid, and burn the entire cache
/// If for some reason we can't access the cache, then this thread self terminates
async fn setup_task_cache(
    cache_key: &str
) -> Result<(String, &Arc<Cache<String, String>>), anyhow::Error> {
    // Self assign an ID for this job
    let id = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);

    match NETWORK_STATE_CACHE.get() {
        Some(cache) =>
            match cache {
                Some(cache) => {
                    let mut jobs: HashMap<String, i8> = HashMap::<String, i8>::new();
                    jobs.insert(id.clone(), 1);

                    cache.insert(
                        cache_key.to_string(),
                        serde_json::to_string(&jobs).unwrap()
                    ).await;
                    return Ok((id, cache));
                }
                None => {
                    return Err(anyhow!("Cache wasn't found."));
                }
            }
        None => {
            return Err(anyhow!("Cache doesn't exist."));
        }
    }
}

/// Returns the mTLS provider
async fn get_mtls_provider() -> Result<MtlsProvider, anyhow::Error> {
    let certificate = match super::credentials::get_credential("certificate".to_string()).await {
        Ok(c) => c,
        Err(_) => {
            return Err(anyhow!("Could not retrieve credential cert"));
        }
    };

    let key = match super::credentials::get_credential("key".to_string()).await {
        Ok(c) => c,
        Err(_) => {
            return Err(anyhow!("Could not retrieve credential key"));
        }
    };

    let ca = match super::credentials::get_credential("ca".to_string()).await {
        Ok(c) => c,
        Err(_) => {
            return Err(anyhow!("Could not retrieve credential ca"));
        }
    };

    match MtlsProvider::new_from_string(ca, certificate, key).await {
        Ok(provider) => Ok(provider),
        Err(e) => Err(anyhow!("{}", e.to_string())),
    }
}

// Generates the QUIC client
async fn get_quic_client(provider: MtlsProvider) -> Result<Client, anyhow::Error> {
    match Client::builder().with_tls(provider) {
        Ok(builder) =>
            match builder.with_io("0.0.0.0:0") {
                Ok(builder) =>
                    match builder.start() {
                        Ok(client) => Ok(client),
                        Err(_) => Err(anyhow!("Could not construct builder")),
                    }
                Err(_) => Err(anyhow!("Could not construct builder")),
            }
        Err(_) => Err(anyhow!("Could not construct builder")),
    }
}

/// Returns the split quic stream
async fn get_stream(client: Client) -> Result<(SendStream, ReceiveStream), anyhow::Error> {
    let quic_connect_string = match
        super::credentials::get_credential("quic_connect_string".to_string()).await
    {
        Ok(c) => c,
        Err(_) => {
            return Err(anyhow!("Could not retrieve credential quic_connect_string"));
        }
    };

    let host = match super::credentials::get_credential("host".to_string()).await {
        Ok(c) => c,
        Err(_) => {
            return Err(anyhow!("Could not retrieve credential host"));
        }
    };

    let connection_string = format!("{}:{}", host, quic_connect_string);
    tracing::info!("{}", connection_string);
    let addr: SocketAddr = match connection_string.parse() {
        Ok(addr) => addr,
        Err(_) => {
            return Err(anyhow!("Could not create socket address"));
        }
    };

    let connect = Connect::new(addr).with_server_name(host);
    let mut connection = match client.connect(connect).await {
        Ok(conn) => conn,
        Err(_) => {
            return Err(anyhow!("Could not create connection"));
        }
    };

    // ensure the connection doesn't time out with inactivity
    match connection.keep_alive(true) {
        Ok(_) => {}
        Err(_) => {
            return Err(anyhow!("Keepalive on connection failed"));
        }
    }

    // open a new stream and split the receiving and sending sides
    let stream = match connection.open_bidirectional_stream().await {
        Ok(stream) => stream,
        Err(_) => {
            return Err(anyhow!("Stream failed to create"));
        }
    };

    _ = stream.connection().keep_alive(true);

    let (receive_stream, send_stream) = stream.split();

    _ = receive_stream.connection().keep_alive(true);
    _ = send_stream.connection().keep_alive(true);

    return Ok((send_stream, receive_stream));
}
