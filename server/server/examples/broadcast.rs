use bytes::Bytes;
use clap::Parser;
use common::Game;
use common::response::ApiConfigResponse;
use common::response::auth::AuthStateResponse;
use common::rustls::MtlsHttpClient;
use common::s2n_quic::{Client, Connection, client::Connect};
use common::structs::channel::{Channel, ChannelEvent, ChannelEvents};
use common::structs::packet::AudioFramePacket;
use common::structs::packet::PacketType;
use common::structs::packet::QuicNetworkPacket;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use hound;
use rodio::Decoder;
use std::io::BufWriter;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
#[cfg(not(target_os = "windows"))]
use std::time::Instant;
use std::{error::Error, net::SocketAddr};
use std::{fs::File, io::BufReader, path::Path};
use x509_parser::prelude::*;

#[derive(Debug, Parser)]
#[clap(about = "Broadcast audio to a BVC server")]
struct Args {
    /// Player name (derived from certificate CN if omitted)
    #[clap(short, long)]
    player: Option<String>,

    /// Path to the audio file to stream
    #[clap(short = 'f', long)]
    audio_file: String,

    /// Server name with optional port (e.g. local.bedrockvc.stream:8444)
    #[clap(short = 'n', long, default_value = "local.bedrockvc.stream")]
    server_name: String,

    /// Game type
    #[clap(short, long, value_enum, default_value = "minecraft")]
    game: Game,

    /// Join or create a group (channel) on the server
    #[clap(long, default_value = "false")]
    group: bool,

    /// Directory holding this client's ca.crt, test.crt and test.key. Defaults to
    /// examples/certs/<hostname>, which allows only one identity per host.
    #[clap(long)]
    certs_dir: Option<String>,

    /// Write received audio to this WAV path. Omitted means received frames are
    /// decoded and discarded, and the receive loop runs until shutdown rather
    /// than stopping at the 1000-packet cap that bounds the file.
    #[clap(long)]
    record: Option<String>,

    /// Connect and receive without ever sending audio. Models a participant who
    /// is present and listening but not talking, which is what fan-out costs are
    /// measured against.
    #[clap(long)]
    listen_only: bool,

    /// Opus frame duration in milliseconds. Sets the encoder frame, the send
    /// cadence and therefore the packet rate: 20ms is 50 packets/sec, 40ms is 25.
    /// Opus permits 10, 20, 40 and 60.
    #[clap(long, default_value = "20")]
    frame_ms: u32,

    /// UDP port to dial instead of the `quic_port` the API reports. The API
    /// reports the port the server bound, so reaching it through a fronting proxy
    /// needs the proxy's port named here.
    #[clap(long)]
    quic_port: Option<u16>,
}

struct Spiral {
    theta: f32,
    step: f32,
}

impl Spiral {
    fn new(step: f32) -> Self {
        Spiral { theta: 0.0, step }
    }
}

impl Iterator for Spiral {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        let radius = 0.1 * (self.theta / 360.0);
        let radians = self.theta.to_radians();
        let x = radius * radians.cos();
        let y = radius * radians.sin();
        self.theta += self.step;
        Some((x, y))
    }
}

fn parse_server_name(server_name: &str) -> (String, u16) {
    if let Some((host, port_str)) = server_name.split_once(':') {
        let port = port_str.parse().unwrap_or(443);
        (host.to_string(), port)
    } else {
        (server_name.to_string(), 443)
    }
}

fn extract_player_from_cert(cert_path: &str) -> Result<String, Box<dyn Error>> {
    let cert_pem = std::fs::read(cert_path)?;
    let (_, pem) = parse_x509_pem(&cert_pem)?;
    let (_, cert) = X509Certificate::from_der(&pem.contents)?;
    let cn_str = cert
        .subject()
        .iter_common_name()
        .next()
        .ok_or("No CommonName found in certificate")?
        .as_str()?
        .to_string();

    let gamertag = match cn_str.split_once(':') {
        Some((_, name)) => name.to_string(),
        None => cn_str,
    };
    Ok(gamertag)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let result = run(args).await;
    println!("{:?}", result);
}

async fn run(args: Args) -> Result<(), Box<dyn Error>> {
    _ = common::s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    let (hostname, api_port) = parse_server_name(&args.server_name);

    let certs_dir = args
        .certs_dir
        .clone()
        .unwrap_or_else(|| format!("{}/examples/certs/{}", env!("CARGO_MANIFEST_DIR"), hostname));
    let ca_path = format!("{}/ca.crt", certs_dir);
    let cert_path = format!("{}/test.crt", certs_dir);
    let key_path = format!("{}/test.key", certs_dir);

    let http_client = MtlsHttpClient::new(&ca_path, &cert_path, &key_path).await?;

    let base_url = format!("https://{}:{}", hostname, api_port);
    println!("Querying API config at {}/api/config", base_url);

    let config: ApiConfigResponse = http_client
        .client()
        .get(format!("{}/api/config", base_url))
        .send()
        .await?
        .json()
        .await?;

    println!(
        "Server config: protocol={}, quic_port={}",
        config.protocol_version, config.quic_port
    );

    // Refresh certificate from server
    println!("Checking certificate state...");
    let auth_state: AuthStateResponse = http_client
        .client()
        .get(format!("{}/api/auth/state", base_url))
        .send()
        .await?
        .json()
        .await?;

    if let (Some(fresh_cert), Some(fresh_key)) =
        (&auth_state.certificate, &auth_state.certificate_key)
    {
        println!("Received fresh certificate from server, updating local certs...");
        std::fs::write(&cert_path, fresh_cert)?;
        std::fs::write(&key_path, fresh_key)?;
    }

    // Derive player name from cert CN (uses latest cert after refresh)
    let id = match args.player {
        Some(name) => name,
        None => {
            let name = extract_player_from_cert(&cert_path)?;
            println!("Derived player name from certificate: {}", name);
            name
        }
    };

    let dial_port = args.quic_port.map(u32::from).unwrap_or(config.quic_port);
    let quic_addr: SocketAddr = tokio::net::lookup_host(format!("{}:{}", hostname, dial_port))
        .await?
        .next()
        .ok_or("Failed to resolve hostname")?;

    println!("Resolved QUIC address: {}", quic_addr);

    // Join or create a group if requested
    let channel_id = if args.group {
        Some(join_or_create_group(http_client.client(), &base_url).await?)
    } else {
        None
    };

    let ca = Path::new(&ca_path);
    let cert = Path::new(&cert_path);
    let key = Path::new(&key_path);

    let provider = common::rustls::MtlsProvider::new(ca, cert, key).await?;

    let dg_endpoint = common::s2n_quic::provider::datagram::default::Endpoint::builder()
        .with_send_capacity(1024)
        .expect("send cap > 0")
        .with_recv_capacity(1024)
        .expect("recv cap > 0")
        .build()
        .expect("build dg endpoint");

    let quic_client = Client::builder()
        .with_tls(provider)?
        .with_io("0.0.0.0:0")?
        .with_datagram(dg_endpoint)?
        .start()?;

    println!("I am client: {} (game: {})", id, args.game.as_str());
    let connect = Connect::new(quic_addr).with_server_name(hostname.clone());
    let mut connection = quic_client.connect(connect).await?;

    connection.keep_alive(true)?;

    let connection = Arc::new(connection);
    let shutdown = Arc::new(AtomicBool::new(false));

    // Ctrl+C handler: signal shutdown and leave channel
    tokio::spawn({
        let shutdown = shutdown.clone();
        let client = http_client.client().clone();
        let base_url = base_url.clone();
        let channel_id = channel_id.clone();
        async move {
            _ = tokio::signal::ctrl_c().await;
            println!("\nCtrl+C received, shutting down...");
            shutdown.store(true, Ordering::SeqCst);

            if let Some(id) = channel_id {
                let leave = ChannelEvent::new(ChannelEvents::Leave);
                let _ = client
                    .put(format!("{}/api/channel/{}", base_url, id))
                    .json(&leave)
                    .send()
                    .await;
                println!("Left channel {}", id);
            }
        }
    });

    let mut tasks = Vec::new();

    tasks.push(tokio::spawn({
        let connection = connection.clone();
        let shutdown = shutdown.clone();
        let record = args.record.clone();
        async move {
            let mut count = 0;
            let mut decoder = opus2::Decoder::new(48000, opus2::Channels::Mono).unwrap();

            let spec = hound::WavSpec {
                channels: 2,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
                sample_rate: 48000,
            };

            let mut wav_writer = match record.as_deref() {
                Some(path) => {
                    let file = File::create(path).unwrap();
                    Some(hound::WavWriter::new(BufWriter::new(file), spec).unwrap())
                }
                None => None,
            };

            let mut last_packet_timestamp: Option<i64> = None;
            let expected_packet_interval_ms = 10;
            let gap_threshold_ms = 50;

            println!("Waiting for incoming datagrams...");
            while let Ok(bytes) = recv_one_datagram(&connection).await {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                match QuicNetworkPacket::from_datagram(&bytes) {
                    Ok(packet) => {
                        if let PacketType::AudioFrame = packet.packet_type {
                            let data: Result<AudioFramePacket, ()> =
                                packet.data.to_owned().try_into();
                            if let Ok(frame) = data {
                                let packet_timestamp = frame.timestamp();

                                if let Some(last_ts) = last_packet_timestamp {
                                    let time_since_last = packet_timestamp - last_ts;
                                    let gap_ms = time_since_last - expected_packet_interval_ms;

                                    if gap_ms > gap_threshold_ms {
                                        println!(
                                            "[GAP DETECTED] Packet {}: {}ms gap detected, inserting {:.2}ms of silence",
                                            count,
                                            time_since_last,
                                            gap_ms
                                        );

                                        if let Some(w) = wav_writer.as_mut() {
                                            let silence_samples_mono =
                                                (gap_ms as f32 * 48.0) as usize;
                                            for _ in 0..silence_samples_mono {
                                                let _ = w.write_sample(0.0f32);
                                                let _ = w.write_sample(0.0f32);
                                            }
                                        }
                                    }
                                }

                                last_packet_timestamp = Some(packet_timestamp);

                                let mut out = vec![0.0; 2880];
                                let out_len =
                                    match decoder.decode_float(&frame.data, &mut out, false) {
                                        Ok(s) => s,
                                        Err(e) => {
                                            println!("Opus decode error: {:?}", e);
                                            0
                                        }
                                    };
                                if out_len > 0 {
                                    if let Some(w) = wav_writer.as_mut() {
                                        for &sample in &out[..out_len] {
                                            let clamped_sample = sample.clamp(-1.0, 1.0);
                                            if let Err(e) = w.write_sample(clamped_sample) {
                                                println!("Wav write sample error: {:?}", e);
                                            }
                                            if let Err(e) = w.write_sample(clamped_sample) {
                                                println!("Wav write sample error: {:?}", e);
                                            }
                                        }
                                    }

                                    count += 1;
                                    if wav_writer.is_some() && count == 1000 {
                                        break;
                                    }
                                    if count % 100 == 0 {
                                        println!("Processed {} packets", count);
                                    }
                                }
                            }
                        }
                        if wav_writer.is_some() && count == 1000 {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Datagram decode error: {:?}", e);
                    }
                }
            }
            println!("Receiving loop ended - processed {} packets", count);

            if let Some(w) = wav_writer {
                if let Err(e) = w.finalize() {
                    println!("Wav write final error: {:?}", e);
                } else {
                    println!("WAV file finalized successfully");
                }
            }
        }
    }));

    let source_file = args.audio_file;
    if args.listen_only {
        println!("listen-only: not sending audio");
    }
    if !args.listen_only {
        tasks.push(tokio::spawn({
        let connection = connection.clone();
        let shutdown = shutdown.clone();
        let source_file = source_file.clone();
        let id = id.clone();
        let frame_ms = args.frame_ms;
        async move {
            // 48 samples per millisecond at 48kHz. The source is read as
            // interleaved stereo and downmixed, so twice this is consumed per frame.
            let frame_samples = 48 * frame_ms as usize;
            let stereo_chunk = frame_samples * 2;
            #[cfg(target_os = "windows")]
            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
            #[cfg(target_os = "windows")]
            windows_targets::link!("winmm.dll" "system" fn timeEndPeriod(uperiod: u32) -> u32);
            #[cfg(target_os = "windows")]
            windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceCounter(lpperformancecount: *mut i64) -> i32);
            #[cfg(target_os = "windows")]
            windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceFrequency(lpfrequency: *mut i64) -> i32);

            #[cfg(target_os = "windows")]
            unsafe {
                timeBeginPeriod(1);
            }

            #[cfg(target_os = "windows")]
            let (frequency, start_time) = {
                let mut frequency = 0i64;
                let mut start_time = 0i64;
                unsafe {
                    QueryPerformanceFrequency(&mut frequency);
                    QueryPerformanceCounter(&mut start_time);
                }
                (frequency, start_time)
            };

            #[cfg(target_os = "windows")]
            let target_interval_ticks = (frequency as f64 * frame_ms as f64 / 1000.0) as i64;

            #[cfg(not(target_os = "windows"))]
            let start_time = Instant::now();
            #[cfg(not(target_os = "windows"))]
            let target_interval = Duration::from_millis(frame_ms as u64);

            let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

            let mut encoder =
                opus2::Encoder::new(48000, opus2::Channels::Mono, opus2::Application::Voip).unwrap();
            _ = encoder.set_bitrate(opus2::Bitrate::Bits(32_000));

            println!("Starting new stream event.");
            let file = BufReader::new(File::open(source_file.clone()).unwrap());
            let source = Decoder::new(file).unwrap();

            let sample_iter = source.into_iter();
            let mut chunk_buffer: Vec<f32> = Vec::with_capacity(stereo_chunk);

            let mut spiral = Spiral::new(0.5);
            println!("Starting streaming playback from file: {}", source_file);
            let mut total_chunks = 0;
            let mut packet_count = 0i64;

            for sample in sample_iter {
                if shutdown.load(Ordering::SeqCst) {
                    println!("Shutdown signal received, stopping send...");
                    break;
                }

                chunk_buffer.push(sample);

                if chunk_buffer.len() < stereo_chunk {
                    continue;
                }

                let chunk_f32: Vec<f32> = chunk_buffer.drain(..stereo_chunk).collect();

                let mono_chunk_f32: Vec<f32> = chunk_f32
                    .chunks_exact(2)
                    .map(|lr| (lr[0] + lr[1]) / 2.0)
                    .collect();

                let mono_chunk: Vec<i16> = mono_chunk_f32
                    .iter()
                    .map(|s| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                    .collect();

                let (_x, _y) = spiral.next().unwrap();
                total_chunks = total_chunks + stereo_chunk;

                let s = encoder.encode_vec(&mono_chunk, frame_samples).unwrap();

                let packet = QuicNetworkPacket {
                    sender: id
                        .parse::<common::PlayerIdentity>()
                        .ok()
                        .map(|identity| {
                            common::structs::packet::PacketSender::player(identity, 0)
                        }),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    data: common::structs::packet::QuicNetworkPacketData::AudioFrame(
                        common::structs::packet::AudioFramePacket::new(
                            s.clone(),
                            None,
                            None,
                        ),
                    ),
                    // Not a server fan-out to one connection, so this envelope carries no sequence.
                    ..Default::default()
                };

                match packet.to_datagram() {
                    Ok(rs) => {
                        if packet_count == 0 {
                            println!("Sending first audio packet...");
                        }
                        let payload = Bytes::from(rs);
                        let send_res = connection.datagram_mut(
                            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                                dg.send_datagram(payload.clone())
                            },
                        );
                        if let Err(e) = send_res {
                            println!("Datagram send query error: {:?}", e);
                        }

                        packet_count += 1;

                        #[cfg(target_os = "windows")]
                        {
                            let target_time = start_time + (packet_count * target_interval_ticks);

                            loop {
                                let mut current_time = 0i64;
                                unsafe {
                                    QueryPerformanceCounter(&mut current_time);
                                }

                                if current_time >= target_time {
                                    break;
                                }

                                let remaining_ticks = target_time - current_time;
                                let remaining_ms = remaining_ticks as f64 * 1000.0 / frequency as f64;

                                if remaining_ms > 2.0 {
                                    tokio::time::sleep(Duration::from_millis((remaining_ms - 1.0) as u64)).await;
                                } else {
                                    tokio::task::yield_now().await;
                                }
                            }
                        }

                        #[cfg(not(target_os = "windows"))]
                        {
                            let target_time = start_time + target_interval * packet_count as u32;

                            loop {
                                let now = Instant::now();
                                if now >= target_time {
                                    break;
                                }

                                let remaining = target_time - now;

                                if remaining > Duration::from_millis(2) {
                                    tokio::time::sleep(remaining - Duration::from_millis(1)).await;
                                } else {
                                    tokio::task::yield_now().await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
            }

            println!("Send task complete");

            #[cfg(target_os = "windows")]
            unsafe {
                timeEndPeriod(1);
            }
        }
    }));
    }

    for task in tasks {
        _ = task.await;
    }

    // Drop connection explicitly to trigger QUIC close handshake
    drop(connection);
    println!("Disconnected");

    Ok(())
}

async fn join_or_create_group(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<String, Box<dyn Error>> {
    println!("Querying channels...");
    let channels: Vec<Channel> = client
        .get(format!("{}/api/channel", base_url))
        .send()
        .await?
        .json()
        .await?;

    let channel_id = if channels.is_empty() {
        println!("No channels found, creating 'broadcast' channel...");
        let id: String = client
            .post(format!("{}/api/channel", base_url))
            .json(&"broadcast")
            .send()
            .await?
            .json()
            .await?;
        println!("Created channel: {}", id);
        id
    } else {
        let channel = &channels[0];
        println!("Found channel: {} ({})", channel.name, channel.id());
        channel.id().to_string()
    };

    println!("Joining channel {}...", channel_id);
    let join_event = ChannelEvent::new(ChannelEvents::Join);
    let response = client
        .put(format!("{}/api/channel/{}", base_url, channel_id))
        .json(&join_event)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Joined channel successfully");
    } else {
        println!("Failed to join channel: {}", response.status());
    }

    Ok(channel_id)
}

struct RecvDatagram<'c> {
    conn: &'c Connection,
}
impl<'c> RecvDatagram<'c> {
    fn new(conn: &'c Connection) -> Self {
        Self { conn }
    }
}
impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, anyhow::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(anyhow::anyhow!(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(anyhow::anyhow!(e))),
        }
    }
}
async fn recv_one_datagram(conn: &Connection) -> Result<Bytes, anyhow::Error> {
    RecvDatagram::new(conn).await
}
