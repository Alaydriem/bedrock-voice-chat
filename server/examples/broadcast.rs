use common::{
    structs::packet::{ DebugPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketCollection },
};
use streamfly::{ certificate::MtlsProvider, new_client };
use tokio::io::AsyncWriteExt;
use std::{ path::Path, time::Duration };
use std::{ error::Error, net::SocketAddr };
use rodio::{ source::SineWave, Source };

const CHANNEL: &str = "BVC_BROADCAST_EXAMPLE_CLIENT";
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = client(
        args[1].to_string().parse::<String>().unwrap(),
        args[2].to_string().parse::<String>().unwrap()
    ).await;
    println!("{:?}", result);
}

async fn client(id: String, channel: String) -> Result<(), Box<dyn Error>> {
    let mut tasks = Vec::new();

    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../certificates/ca.crt");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test.crt");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test.key");
    let ca = Path::new(ca_path);
    let cert = Path::new(cert_path);
    let key = Path::new(key_path);
    let provider = MtlsProvider::new(ca, cert, key).await;

    let mut client = new_client(
        "127.0.0.1:3001".parse().unwrap(),
        "localhost",
        provider.unwrap()
    ).await.unwrap();

    // spawn a task that copies responses from the server to stdout
    tasks.push(
        tokio::spawn(async move {
            let mut packet = Vec::<u8>::new();

            let rx = client.subscribe(&channel).await.unwrap();

            let (_, mut reader) = rx.recv().await.unwrap();
            while let Ok(Some(data)) = reader.receive().await {
                packet.append(&mut data.to_vec());

                match QuicNetworkPacket::from_stream(&mut packet) {
                    Ok(packets) => {
                        for packet in packets {
                            println!("Received [{}] from [{}]", packet.sequence_id, packet.author);
                        }
                    }
                    Err(_) => {
                        println!("Failed to process stream.");
                    }
                };
            }
            println!("Recieving loop died");
        })
    );

    let provider = MtlsProvider::new(ca, cert, key).await;
    let mut client = new_client(
        "127.0.0.1:3001".parse().unwrap(),
        "localhost",
        provider.unwrap()
    ).await.unwrap();

    tasks.push(
        tokio::spawn(async move {
            let id = id.clone();
            let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

            let mut encoder = opus::Encoder
                ::new(48000, opus::Channels::Mono, opus::Application::Voip)
                .unwrap();

            _ = encoder.set_bitrate(opus::Bitrate::Bits(64_000));
            let source = SineWave::new(440.0)
                .take_duration(Duration::from_secs_f32(0.02))
                .amplify(0.01);

            let (_, mut writer) = client.open_stream(&id.clone()).await.unwrap();

            let mut count = 0;
            loop {
                count = count + 1;
                let now = std::time::Instant::now();
                let source = source.clone();
                let s: Vec<f32> = source.collect();

                let s = encoder.encode_vec_float(&s, s.len() * 4).unwrap();
                let packet = QuicNetworkPacket {
                    client_id: client_id.clone(),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    author: id.clone(),
                    sequence_id: count,
                    data: common::structs::packet::QuicNetworkPacketData::AudioFrame(
                        common::structs::packet::AudioFramePacket {
                            length: s.len(),
                            data: s.clone(),
                            sample_rate: 48000,
                            author: id.clone(),
                            coordinate: None,
                        }
                    ),
                };

                match packet.to_vec() {
                    Ok(rs) => {
                        _ = writer.write_all(&rs).await;
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
