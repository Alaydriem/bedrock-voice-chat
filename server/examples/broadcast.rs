use bytes::Bytes;
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
use s2n_quic::{client::Connect, Client, Connection};
use std::io::BufWriter;
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, net::SocketAddr};
use std::{fs::File, io::BufReader, path::Path};

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

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = client(
        args[1].to_string().parse::<String>().unwrap(),
        args[2].to_string().parse::<String>().unwrap(),
        args[3].to_string().parse::<String>().unwrap(),
        args[4].to_string().parse::<String>().unwrap(),
    )
    .await;
    println!("{:?}", result);
}

async fn client(
    id: String,
    source_file: String,
    socket_addr: String,
    server_name: String,
) -> Result<(), Box<dyn Error>> {
    _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/ca.crt");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/test.crt");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/test.key");

    let ca = Path::new(ca_path);
    let cert = Path::new(cert_path);
    let key = Path::new(key_path);

    let provider = common::rustls::MtlsProvider::new(ca, cert, key).await?;

    let dg_endpoint = s2n_quic::provider::datagram::default::Endpoint::builder()
        .with_send_capacity(1024)
        .expect("send cap > 0")
        .with_recv_capacity(1024)
        .expect("recv cap > 0")
        .build()
        .expect("build dg endpoint");

    let client = Client::builder()
        .with_tls(provider)?
        .with_io("0.0.0.0:0")?
        .with_datagram(dg_endpoint)?
        .start()?;

    println!("I am client: {}", id);
    let addr: SocketAddr = socket_addr.parse()?;
    let connect = Connect::new(addr).with_server_name(server_name);
    let mut connection = client.connect(connect).await?;

    connection.keep_alive(true)?;

    let connection = Arc::new(connection);

    let mut tasks = Vec::new();
    tasks.push(tokio::spawn({
        let connection = connection.clone();
        async move {
            let mut count = 0;
            let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();

            let spec = hound::WavSpec {
                channels: 2,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
                sample_rate: 48000,
            };

            let file = File::create("C:\\Users\\charl\\Downloads\\sample_voice.wav").unwrap();
            let writer = BufWriter::new(file);
            let mut wav_writer = hound::WavWriter::new(writer, spec).unwrap();

            while let Ok(bytes) = recv_one_datagram(&connection).await {
                match QuicNetworkPacket::from_datagram(&bytes) {
                    Ok(packet) => {
                        if let PacketType::AudioFrame = packet.packet_type {
                            let data: Result<AudioFramePacket, ()> =
                                packet.data.to_owned().try_into();
                            if let Ok(frame) = data {
                                let mut out = vec![0.0; 960];
                                let out_len =
                                    match decoder.decode_float(&frame.data, &mut out, false) {
                                        Ok(s) => s,
                                        Err(e) => {
                                            println!("Opus decode error: {:?}", e);
                                            0
                                        }
                                    };
                                if out_len > 0 {
                                    for &sample in &out {
                                        let clamped_sample = sample.clamp(-1.0, 1.0);
                                        if let Err(e) = wav_writer.write_sample(clamped_sample) {
                                            println!("Wav write sample error: {:?}", e);
                                        }
                                    }
                                    if count == 1000 {
                                        break;
                                    }
                                    count += 1;
                                    if count % 100 == 0 {
                                        println!("{:?}", count);
                                    }
                                }
                            }
                        }
                        if count == 1000 {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Datagram decode error: {:?}", e);
                    }
                }
            }
            println!("Receiving loop ended");

            if let Err(e) = wav_writer.finalize() {
                println!("Wav write final error: {:?}", e);
            } else {
                println!("WAV file finalized successfully");
            }
        }
    }));

    tasks.push(tokio::spawn({
        let connection = connection.clone();
        async move {
            // Windows high-resolution timing setup
            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
            windows_targets::link!("winmm.dll" "system" fn timeEndPeriod(uperiod: u32) -> u32);
            windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceCounter(lpperformancecount: *mut i64) -> i32);
            windows_targets::link!("kernel32.dll" "system" fn QueryPerformanceFrequency(lpfrequency: *mut i64) -> i32);

            unsafe {
                timeBeginPeriod(1);
            }

            // Get high-resolution timer frequency
            let mut frequency = 0i64;
            let mut start_time = 0i64;
            unsafe {
                QueryPerformanceFrequency(&mut frequency);
                QueryPerformanceCounter(&mut start_time);
            }

            let target_interval_ms = 20.0;
            let target_interval_ticks = (frequency as f64 * target_interval_ms / 1000.0) as i64;

            let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

            let mut encoder =
                opus::Encoder::new(48000, opus::Channels::Stereo, opus::Application::Voip).unwrap();

            _ = encoder.set_force_channels(Some(opus::Channels::Mono));
            _ = encoder.set_bitrate(opus::Bitrate::Bits(32_000));

            println!("Starting new stream event.");
            let file = BufReader::new(File::open(source_file.clone()).unwrap());
            let source = Decoder::new(file).unwrap();

            let sample_iter = source.into_iter();
            let mut chunk_buffer: Vec<f32> = Vec::with_capacity(1920);

            let mut spiral = Spiral::new(0.5);
            println!("Starting streaming playback from file: {}", source_file);
            let mut total_chunks = 0;
            let mut packet_count = 0i64;

            for sample in sample_iter {
                chunk_buffer.push(sample);

                if chunk_buffer.len() < 1920 {
                    continue;
                }

                let chunk_f32: Vec<f32> = chunk_buffer.drain(..1920).collect();
                let chunk: Vec<i16> = chunk_f32
                    .iter()
                    .map(|s| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                    .collect();
                let (_x, _y) = spiral.next().unwrap();
                total_chunks = total_chunks + 1920;
                let s = encoder.encode_vec(&chunk, 960).unwrap();
                let packet = QuicNetworkPacket {
                    owner: Some(common::structs::packet::PacketOwner {
                        name: id.clone(),
                        client_id: client_id.clone(),
                    }),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    data: common::structs::packet::QuicNetworkPacketData::AudioFrame(
                        common::structs::packet::AudioFramePacket::new(
                            s.clone(),
                            48000,
                            Some(common::Coordinate {
                                x: 335.0,
                                y: 78.0,
                                z: -689.0,
                            }),
                            None,
                            Some(common::Dimension::Overworld),
                            None,
                        ),
                    ),
                };

                match packet.to_datagram() {
                    Ok(rs) => {
                        let payload = Bytes::from(rs);
                        let send_res = connection.datagram_mut(
                            |dg: &mut s2n_quic::provider::datagram::default::Sender| {
                                dg.send_datagram(payload.clone())
                            },
                        );
                        if let Err(e) = send_res {
                            println!("Datagram send query error: {:?}", e);
                        }

                        // High-resolution timing instead of tokio::time::sleep
                        packet_count += 1;
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
                                // Use tokio sleep for longer waits to avoid spinning
                                tokio::time::sleep(Duration::from_millis((remaining_ms - 1.0) as u64)).await;
                            } else {
                                // Precise spinning for the final 1-2ms
                                tokio::task::yield_now().await;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
            }

            if !chunk_buffer.is_empty() {
                println!(
                    "Sending final partial chunk with {} samples",
                    chunk_buffer.len()
                );
                chunk_buffer.resize(960, 0.0f32);

                let (_x, _y) = spiral.next().unwrap();
                let final_chunk: Vec<i16> = chunk_buffer
                    .iter()
                    .map(|s| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
                    .collect();
                let s = encoder
                    .encode_vec(&final_chunk, final_chunk.len() * 4)
                    .unwrap();
                let packet = QuicNetworkPacket {
                    owner: Some(common::structs::packet::PacketOwner {
                        name: id.clone(),
                        client_id: client_id.clone(),
                    }),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    data: common::structs::packet::QuicNetworkPacketData::AudioFrame(
                        common::structs::packet::AudioFramePacket::new(
                            s.clone(),
                            48000,
                            Some(common::Coordinate {
                                x: 335.0,
                                y: 78.0,
                                z: -689.0,
                            }),
                            None,
                            Some(common::Dimension::Overworld),
                            None,
                        ),
                    ),
                };

                match packet.to_datagram() {
                    Ok(rs) => {
                        let payload = Bytes::from(rs);
                        let send_res = connection.datagram_mut(
                            |dg: &mut s2n_quic::provider::datagram::default::Sender| {
                                dg.send_datagram(payload.clone())
                            },
                        );
                        if let Err(e) = send_res {
                            println!("Datagram send query error: {:?}", e);
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
            println!("Send task complete");

            // Clean up Windows timer resolution
            unsafe {
                timeEndPeriod(1);
            }
        }
    }));

    for task in tasks {
        _ = task.await;
    }

    Ok(())
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
        match self
            .conn
            .datagram_mut(|r: &mut s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            }) {
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
