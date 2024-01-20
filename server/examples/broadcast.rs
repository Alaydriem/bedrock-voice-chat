use common::{
    mtlsprovider::MtlsProvider,
    structs::packet::{ DebugPacket, PacketType, QuicNetworkPacket, QUICK_NETWORK_PACKET_HEADER },
};
use s2n_quic::{ client::Connect, Client };
use std::path::Path;
use std::{ error::Error, net::SocketAddr };
use rodio::Source;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = client(args[1].to_string().parse::<String>().unwrap()).await;
    println!("{:?}", result);
}

async fn client(id: String) -> Result<(), Box<dyn Error>> {
    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../certificates/ca.crt");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test.crt");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test.key");

    let ca = Path::new(ca_path);
    let cert = Path::new(cert_path);
    let key = Path::new(key_path);

    let provider = MtlsProvider::new(ca, cert, key).await?;

    let client = Client::builder().with_tls(provider)?.with_io("0.0.0.0:0")?.start()?;

    println!("I am client: {}", id);
    let addr: SocketAddr = "127.0.0.1:3001".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
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
    tasks.push(
        tokio::spawn(async move {
            let magic_header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
            let mut packet = Vec::<u8>::new();
            let mut packet_len_total: usize = 0;

            while let Ok(Some(data)) = receive_stream.receive().await {
                tracing::info!("Received data.");
                packet_len_total = packet_len_total + data.to_vec().len();
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
                    let packet_copy = packet.clone();
                    let packet_to_process = packet_copy
                        .get(0..packet_len + 13)
                        .unwrap()
                        .to_vec();

                    packet = packet
                        .get(packet_len + 13..packet.len())
                        .unwrap()
                        .to_vec();

                    // Strip the header and frame length
                    let packet_to_process = packet_to_process
                        .get(13..packet_to_process.len())
                        .unwrap();

                    match QuicNetworkPacket::from_vec(&packet_to_process) {
                        Ok(packet) => {
                            match packet.packet_type {
                                PacketType::AudioFrame => {}
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
                    };
                }
            }
            println!("Recieving loop died");
        })
    );

    tasks.push(
        tokio::spawn(async move {
            let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

            let source = rodio::source::SineWave
                ::new(440.0)
                .take_duration(std::time::Duration::from_secs_f32(0.02))
                .amplify(0.2);

            let s: Vec<f32> = source.collect();

            let mut encoder = opus::Encoder
                ::new(48000, opus::Channels::Mono, opus::Application::Voip)
                .unwrap();
            _ = encoder.set_bitrate(opus::Bitrate::Bits(32_000));

            loop {
                let s = encoder.encode_vec_float(&s, s.len() * 4).unwrap();
                let packet = QuicNetworkPacket {
                    client_id: client_id.clone(),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    author: id.clone(),
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
                    Ok(reader) => {
                        let result = send_stream.send(reader.into()).await;
                        _ = send_stream.flush().await;
                        if result.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            }

            println!("Sending loop died");
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
