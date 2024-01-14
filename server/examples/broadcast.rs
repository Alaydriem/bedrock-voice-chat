use std::{ error::Error, net::SocketAddr };
use bytes::Bytes;
use common::{
    mtlsprovider::MtlsProvider,
    structs::packet::{ QuicNetworkPacket, PacketType, QUICK_NETWORK_PACKET_HEADER },
};
use s2n_quic::{ client::Connect, Client };
use std::path::Path;

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
            // Inbound packets may be split across multiple receive() calls, so we need to rejoin them
            // Packets may arrive in a different order we sent them, but the packet itself should arrive serially in receive() calls
            let magic_header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
            let mut packet = Vec::<u8>::new();

            loop {
                let mut chunks = [Bytes::new()];
                match receive_stream.receive_vectored(&mut chunks).await {
                    Ok((count, is_open)) => {
                        // If the connection closes, then we can terminate the reader loop
                        if !is_open {
                            tracing::info!("Stream closed.");
                            break;
                        }

                        for chunk in &chunks[..count] {
                            packet.append(&mut chunk.to_vec());
                        }

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

                        let packet_len = usize::from_be_bytes(
                            packet_length.unwrap().try_into().unwrap()
                        );

                        if packet_header.eq(&magic_header) && packet.len() == packet_len + 13 {
                            let packet_to_process = packet.clone();
                            let packet_to_process = packet_to_process.get(13..packet.len());

                            packet = Vec::<u8>::new();

                            if packet_to_process.is_none() {
                                tracing::error!(
                                    "RON serialized packet data length mismatch after verifier."
                                );
                                continue;
                            }

                            match QuicNetworkPacket::from_vec(packet_to_process.unwrap()) {
                                Ok(packet) => {
                                    println!("Got data packet in loop");
                                    match packet.packet_type {
                                        PacketType::AudioFrame => {
                                            println!("Got Audio Frame");
                                        }
                                        PacketType::Positions => {
                                            let data = packet.data
                                                .as_any()
                                                .downcast_ref::<common::structs::packet::PlayerDataPacket>()
                                                .unwrap();
                                            dbg!("{:?}", data);
                                        }
                                        PacketType::Debug => {
                                            let data = packet.data
                                                .as_any()
                                                .downcast_ref::<common::structs::packet::DebugPacket>()
                                                .unwrap();
                                            dbg!("{:?}", data);
                                        }
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
                        // Otherwise we need to collect more data.
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                        break;
                    }
                }
            }
            println!("Recieving loop died");
        })
    );

    tasks.push(
        tokio::spawn(async move {
            let client_id: Vec<u8> = (0..32).map(|_| { rand::random::<u8>() }).collect();

            loop {
                let packet = QuicNetworkPacket {
                    client_id: client_id.clone(),
                    packet_type: common::structs::packet::PacketType::Debug,
                    author: id.clone(),
                    data: Box::new(common::structs::packet::DebugPacket(id.clone())),
                };

                match packet.to_vec() {
                    Ok(reader) => {
                        let result = send_stream.send(reader.into()).await;
                        if result.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            println!("Sending loop died");
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
