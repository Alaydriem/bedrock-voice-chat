use common::structs::packet::AudioFramePacket;
use common::structs::packet::PacketType;
use common::structs::packet::QuicNetworkPacket;
use rodio::Decoder;
use s2n_quic::{client::Connect, Client};
use std::time::Duration;
use std::{error::Error, net::SocketAddr};
use std::{fs::File, io::BufReader, path::Path};
use tokio::io::AsyncWriteExt;
use hound;
use std::io::BufWriter;

struct Spiral {
    theta: f32, // Angle in degrees
    step: f32,  // Step increment for theta
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
        args[1].to_string().parse::<String>().unwrap(), // Player Identifier
        args[2].to_string().parse::<String>().unwrap(), // Path to Source File
        args[3].to_string().parse::<String>().unwrap(), // Host:Port of QUIC Server
        args[4].to_string().parse::<String>().unwrap(), // Server Name
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

    let client = Client::builder()
        .with_tls(provider)?
        .with_io("0.0.0.0:0")?
        .start()?;

    println!("I am client: {}", id);
    let addr: SocketAddr = socket_addr.parse()?;
    let connect = Connect::new(addr).with_server_name(server_name);
    let mut connection = client.connect(connect).await?;

    // ensure the connection doesn't time out with inactivity
    connection.keep_alive(true)?;

    // open a new stream and split the receiving and sending sides
    let stream = connection.open_bidirectional_stream().await?;
    _ = stream.connection().keep_alive(true);

    let (mut receive_stream, mut send_stream) = stream.split();

    _ = receive_stream.connection().keep_alive(true);
    _ = send_stream.connection().keep_alive(true);

    let mut tasks = Vec::new();

    // spawn a task that copies responses from the server to stdout

    tasks.push(tokio::spawn(async move {
        let mut packet = Vec::<u8>::new();
        let mut count = 0;
        let mut decoder = opus::Decoder::new(48000, opus::Channels::Mono).unwrap();
    
        let spec = hound::WavSpec {
            channels: 2,
            bits_per_sample: 32, // Change to 16 for compatibility
            sample_format: hound::SampleFormat::Float,
            sample_rate: 48000,
        };
    
        let file = File::create("C:\\Users\\charl\\Downloads\\sample_voice.wav").unwrap();
        let writer = BufWriter::new(file);
        let mut wav_writer = hound::WavWriter::new(writer, spec).unwrap();
    
        while let Ok(Some(data)) = receive_stream.receive().await {
            packet.append(&mut data.to_vec());
    
            match QuicNetworkPacket::from_stream(&mut packet) {
                Ok(packets) => {
                    for packet in packets {
                        match packet.packet_type {
                            PacketType::AudioFrame => {
                                let data: Result<AudioFramePacket, ()> = packet.data.to_owned().try_into();
                                let mut out = vec![0.0; 960];
                                let out_len = match decoder.decode_float(&data.unwrap().data, &mut out, false) {
                                    Ok(s) => {
                                        s
                                    }
                                    Err(e) => {
                                        println!("Opus decode error: {:?}", e); // Debug error message
                                        0
                                    }
                                };
    
                                if out_len > 0 {
                                    // Write to the WAV file with clamping
                                    for &sample in &out {
                                        let clamped_sample = sample.clamp(-1.0, 1.0); // Ensure sample is in range [-1.0, 1.0]
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
                            _ => {}
                        }
                    }

                    if count == 1000 {                                        
                        break;
                    }

                }
                Err(e) => {
                    println!("{:?}", e);
                }
            };
        }
        println!("Receiving loop died");
    
        if let Err(e) = wav_writer.finalize() {
            println!("Wav write final error: {:?}", e);
        } else {
            println!("WAV file finalized successfully");
        }
    }));

    tasks.push(tokio::spawn(async move {
        // Windows has a sleep resolution time of ~15.6ms, which is much longer than the 5-9ms it takes to generate a "real" packet
        // This simulates the slow generation without generating packets in us time.
        windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
        unsafe {
            timeBeginPeriod(1);
        }

        let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

        let mut encoder =
            opus::Encoder::new(48000, opus::Channels::Mono, opus::Application::Voip).unwrap();

        _ = encoder.set_bitrate(opus::Bitrate::Bits(32_000));

        println!("Starting new stream event.");
        let file = BufReader::new(File::open(source_file.clone()).unwrap());
        let source = Decoder::new(file).unwrap();
        
        // TRUE STREAMING: Process samples as iterator without collecting entire file
        let sample_iter = source.into_iter();
        let mut chunk_buffer = Vec::with_capacity(480);

        let mut spiral = Spiral::new(0.5);
        println!("Starting streaming playback from file: {}", source_file);
        let mut total_chunks = 0;
        
        // Process samples one by one, building 480-sample chunks
        for sample in sample_iter {
            chunk_buffer.push(sample);
            
            // Continue if we don't have a full chunk yet
            if chunk_buffer.len() < 480 {
                continue;
            }
            
            // Process the full 480-sample chunk exactly like original
            let chunk: Vec<i16> = chunk_buffer.drain(..480).collect();
            let (_x, _y) = spiral.next().unwrap();
            total_chunks = total_chunks + 480;
            let s = encoder.encode_vec(&chunk, chunk.len() * 4).unwrap();
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
                        None
                    ),
                ),
            };

            match packet.to_vec() {
                Ok(rs) => {
                    _ = send_stream.write_all(&rs).await;
                    // This should be 20ms of audio
                    if total_chunks % 9600 == 0 {
                        _ = send_stream.flush().await;
                        _ = tokio::time::sleep(Duration::from_millis(60)).await;
                    }
                }
                Err(e) => {
                    println!("{}", e.to_string());
                }
            }
        }
        
        // Handle any remaining samples in chunk_buffer (less than 480)
        if !chunk_buffer.is_empty() {
            println!("Sending final partial chunk with {} samples", chunk_buffer.len());
            // Pad to 480 samples with silence to match original behavior
            chunk_buffer.resize(480, 0);
            
            let (_x, _y) = spiral.next().unwrap();
            let s = encoder.encode_vec(&chunk_buffer, chunk_buffer.len() * 4).unwrap();
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
                        None
                    ),
                ),
            };

            match packet.to_vec() {
                Ok(rs) => {
                    _ = send_stream.write_all(&rs).await;
                    _ = send_stream.flush().await;
                }
                Err(e) => {
                    println!("{}", e.to_string());
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
        let r = send_stream.close().await;
        println!("Close Sending Stream {:?}", r);
    }));

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
