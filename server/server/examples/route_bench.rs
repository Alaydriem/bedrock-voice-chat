//! In-process routing capacity benchmark.
//!
//! Drives ConnectionRegistry::route_audio_frame directly — no QUIC, TLS, or
//! UDP — so the numbers isolate the routing hot path on the machine it runs
//! on. Speakers run as independent tokio tasks, mirroring the production
//! per-connection input tasks, so multi-speaker runs also measure multi-core
//! scaling.
//!
//!   cargo run --release --example route_bench -- --connections 50 --speakers 50
//!   cargo run --release --example route_bench -- --connections 50 --speakers 15 --group-size 5 --channels

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use bvc_server_lib::stream::quic::connection_registry::{ConnectionRegistry, RoutedPacket};
use clap::Parser;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::{
    AudioFramePacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, Orientation, PlayerEnum};
use moka::future::Cache;
use tokio::sync::mpsc;

#[derive(Debug, Parser)]
#[clap(about = "Benchmark ConnectionRegistry::route_audio_frame on this machine")]
struct Args {
    /// Registered (listening) connections
    #[clap(long, default_value = "50")]
    connections: usize,

    /// Concurrent speaker tasks (each is one connection's input task)
    #[clap(long, default_value = "10")]
    speakers: usize,

    /// Players per spatial cluster; clusters are placed out of range of each
    /// other, modelling groups scattered across the world
    #[clap(long, default_value = "5")]
    group_size: usize,

    /// Also join each cluster into a shared voice channel
    #[clap(long, default_value = "false")]
    channels: bool,

    /// Pace speakers at the production 20ms frame cadence instead of
    /// routing as fast as possible (saturation mode)
    #[clap(long, default_value = "false")]
    paced: bool,

    /// Seconds to run
    #[clap(long, default_value = "10")]
    duration: u64,

    /// Server broadcast_range config value
    #[clap(long, default_value = "50.0")]
    broadcast_range: f32,
}

struct RouteBench {
    args: Args,
    registry: Arc<ConnectionRegistry>,
    player_cache: Arc<Cache<String, PlayerEnum>>,
    delivered: Arc<AtomicU64>,
}

impl RouteBench {
    fn new(args: Args) -> Self {
        Self {
            args,
            registry: Arc::new(ConnectionRegistry::new()),
            player_cache: Arc::new(Cache::builder().build()),
            delivered: Arc::new(AtomicU64::new(0)),
        }
    }

    fn player_name(i: usize) -> String {
        format!("Player{:03}", i)
    }

    // Clusters sit 10km apart (far outside 1.73 * range); members within a
    // cluster sit 5 blocks apart (well inside range).
    fn player(&self, i: usize) -> PlayerEnum {
        let cluster = (i / self.args.group_size.max(1)) as f32;
        let offset = (i % self.args.group_size.max(1)) as f32;
        PlayerEnum::Minecraft(MinecraftPlayer {
            name: Self::player_name(i),
            coordinates: Coordinate {
                x: cluster * 10_000.0 + offset * 5.0,
                y: 64.0,
                z: 0.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: None,
        })
    }

    fn audio_packet(&self, i: usize, sender: PlayerEnum) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: Some(PacketOwner {
                name: Self::player_name(i),
                client_id: vec![i as u8, (i >> 8) as u8],
            }),
            // 160 bytes ~= one 20ms Opus frame at 64kbps
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
                48000,
                Some(sender),
                Some(true),
            )),
        }
    }

    async fn setup(&self) {
        for i in 0..self.args.connections {
            let player = self.player(i);
            self.player_cache
                .insert(Self::player_name(i), player)
                .await;

            let (tx, mut rx) = mpsc::channel::<RoutedPacket>(500);
            self.registry
                .register(vec![i as u8, (i >> 8) as u8], Self::player_name(i), tx);

            let delivered = self.delivered.clone();
            tokio::spawn(async move {
                while rx.recv().await.is_some() {
                    delivered.fetch_add(1, Ordering::Relaxed);
                }
            });

            if self.args.channels {
                let cluster = i / self.args.group_size.max(1);
                self.registry.update_player_channel(
                    format!("minecraft:{}", Self::player_name(i)),
                    format!("chan{}", cluster),
                );
            }
        }
    }

    async fn run(self) {
        self.setup().await;

        let deadline = Instant::now() + Duration::from_secs(self.args.duration);
        let mut handles = Vec::with_capacity(self.args.speakers);

        for i in 0..self.args.speakers {
            let registry = self.registry.clone();
            let cache = self.player_cache.clone();
            let sender = self.player(i);
            let packet = self.audio_packet(i, sender);
            let range = self.args.broadcast_range;
            let paced = self.args.paced;

            handles.push(tokio::spawn(async move {
                let mut latencies_ns: Vec<u64> = Vec::with_capacity(1_000_000);
                let mut ticker = tokio::time::interval(Duration::from_millis(20));
                while Instant::now() < deadline {
                    if paced {
                        ticker.tick().await;
                    }
                    let started = Instant::now();
                    registry.route_audio_frame(&packet, &cache, range, 5.0).await;
                    latencies_ns.push(started.elapsed().as_nanos() as u64);
                }
                latencies_ns
            }));
        }

        let started = Instant::now();
        let mut all_latencies: Vec<u64> = Vec::new();
        for handle in handles {
            if let Ok(mut l) = handle.await {
                all_latencies.append(&mut l);
            }
        }
        let elapsed = started.elapsed().as_secs_f64().max(0.001);

        self.report(all_latencies, elapsed);
    }

    fn report(&self, mut latencies_ns: Vec<u64>, elapsed_secs: f64) {
        latencies_ns.sort_unstable();
        let frames = latencies_ns.len() as f64;
        let delivered = self.delivered.load(Ordering::Relaxed);

        println!("--- route_bench results ---");
        println!(
            "connections={} speakers={} group_size={} channels={} paced={} duration={}s",
            self.args.connections,
            self.args.speakers,
            self.args.group_size,
            self.args.channels,
            self.args.paced,
            self.args.duration,
        );
        println!("frames routed:      {}", latencies_ns.len());
        println!("frames/sec:         {:.0}", frames / elapsed_secs);
        println!("deliveries:         {}", delivered);
        println!("deliveries/sec:     {:.0}", delivered as f64 / elapsed_secs);
        println!(
            "route call µs:      p50={:.1} p95={:.1} p99={:.1} max={:.1}",
            Self::percentile(&latencies_ns, 0.50) / 1_000.0,
            Self::percentile(&latencies_ns, 0.95) / 1_000.0,
            Self::percentile(&latencies_ns, 0.99) / 1_000.0,
            *latencies_ns.last().unwrap_or(&0) as f64 / 1_000.0,
        );
    }

    fn percentile(sorted_ns: &[u64], q: f64) -> f64 {
        if sorted_ns.is_empty() {
            return 0.0;
        }
        let idx = ((sorted_ns.len() as f64 - 1.0) * q).round() as usize;
        sorted_ns[idx] as f64
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    RouteBench::new(args).run().await;
}
