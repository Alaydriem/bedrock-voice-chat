use crate::config::ApplicationConfig;
use anyhow::anyhow;
use async_once_cell::OnceCell;
use common::structs::packet::{
    AudioFramePacket,
    PacketType,
    PlayerDataPacket,
    QuicNetworkPacket,
    QuicNetworkPacketData,
};
use anyhow::Result;
use bytes::Bytes;
use futures::{ future::{ self, BoxFuture }, FutureExt };
use streamfly::{ certificate::MtlsProvider, serve };

use moka::sync::Cache;
use std::collections::hash_map::RandomState;
use std::path::Path;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

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
                moka::sync::Cache
                    ::builder()
                    .time_to_live(Duration::from_secs(300))
                    .max_capacity(100)
                    .build()
            )
        );
    }).await;

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

    let mut tasks = Vec::new();

    match serve(io_connection_str, provider, mutator).await {
        Ok(listener) => tasks.push(listener),
        Err(e) => {
            return Err(anyhow!("Unable to start pub/sub QUIC listener {}", e.to_string()));
        }
    }

    let cache = match get_cache() {
        Ok(cache) => cache,
        Err(e) => {
            tracing::error!(
                "A cache was constructed but didn't survive. This shouldn't happen. Restart bvc server. {}",
                e.to_string()
            );
            return Err(e);
        }
    };

    tasks.push(
        tokio::spawn(async move {
            let cache = cache.clone();
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
                                            cache.insert(player.name.clone(), player.clone());
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

    return Ok(tasks);
}

/// Mutates the incoming QuicNetworkPackets to append the player location to them, if they exist
fn mutator(mut data: &mut Vec<u8>) -> BoxFuture<'static, Result<Bytes, ()>> {
    let dc = data.clone();
    let cache = match get_cache() {
        Ok(cache) => cache,
        Err(_) => {
            // If we can't retrieve the cache, return the raw data back
            return future::ready(Ok(dc.try_into().unwrap())).boxed();
        }
    };

    match QuicNetworkPacket::from_stream(&mut data) {
        Ok(mut packets) => {
            let mut stream_packets = Vec::new();
            for mut packet in packets.iter_mut() {
                let pt = packet.packet_type.clone();
                if pt.eq(&PacketType::AudioFrame) {
                    update_packet_with_player_coordinates(&mut packet, cache.clone());
                }

                match packet.to_vec() {
                    Ok(mut ds) => stream_packets.append(&mut ds),
                    Err(_) => {}
                };
            }

            let bytes: Bytes = stream_packets.try_into().unwrap();
            return future::ready(Ok(bytes)).boxed();
        }
        Err(_) => {
            return future::ready(Ok(dc.try_into().unwrap())).boxed();
        }
    }
}

/// Returns the cache object without if match branching nonsense
fn get_cache() -> Result<Arc<Cache<String, common::Player>>, anyhow::Error> {
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
fn update_packet_with_player_coordinates(
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
                                    match cache.get(&data.author) {
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
