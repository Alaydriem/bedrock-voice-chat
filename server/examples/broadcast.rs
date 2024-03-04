use common::{ mtlsprovider::MtlsProvider, structs::packet::QuicNetworkPacket };
use s2n_quic::{ client::Connect, Client };
use tokio::io::AsyncWriteExt;
use std::{ fs::File, io::BufReader, path::Path };
use std::{ error::Error, net::SocketAddr };
use rodio::Decoder;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = client(
        args[1].to_string().parse::<String>().unwrap(),
        args[2].to_string().parse::<String>().unwrap()
    ).await;
    println!("{:?}", result);
}

async fn client(id: String, source_file: String) -> Result<(), Box<dyn Error>> {
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

    let (receive_stream, mut send_stream) = stream.split();

    _ = receive_stream.connection().keep_alive(true);
    _ = send_stream.connection().keep_alive(true);

    let mut tasks = Vec::new();

    // spawn a task that copies responses from the server to stdout
    /*
    tasks.push(
        tokio::spawn(async move {
            let mut packet = Vec::<u8>::new();

            while let Ok(Some(data)) = receive_stream.receive().await {
                packet.append(&mut data.to_vec());

                match QuicNetworkPacketCollection::from_stream(&mut packet) {
                    Ok(packets) => {
                        for p in packets {
                            let sourced_from = p.frames
                                .iter()
                                .map(|f| f.author.clone())
                                .collect::<String>();
                            if sourced_from.len() != 0 {
                                println!("Got a frame collection back from {:?}", sourced_from);
                            }
                        }
                    }
                    Err(_) => {}
                };
            }
            println!("Recieving loop died");
        })
    );
    */

    tasks.push(
        tokio::spawn(async move {
            // Windows has a sleep resolution time of ~15.6ms, which is much longer than the 5-9ms it takes to generate a "real" packet
            // This simulates the slow generation without generating packets in us time.
            windows_targets::link!("winmm.dll" "system" fn timeBeginPeriod(uperiod: u32) -> u32);
            unsafe {
                timeBeginPeriod(1);
            }

            let client_id: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();

            let mut encoder = opus::Encoder
                ::new(48000, opus::Channels::Mono, opus::Application::Voip)
                .unwrap();

            _ = encoder.set_bitrate(opus::Bitrate::Bits(64_000));

            println!("Starting new stream event.");
            let file = BufReader::new(File::open(source_file.clone()).unwrap());
            let source = Decoder::new(file).unwrap();
            let ss: Vec<i16> = source.collect();

            for chunk in ss.chunks(480) {
                if chunk.len() < 480 {
                    break;
                }
                let s = encoder.encode_vec(chunk, chunk.len() * 4).unwrap();
                let packet = QuicNetworkPacket {
                    client_id: client_id.clone(),
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    author: id.clone(),
                    in_group: None,
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
                        _ = send_stream.write_all(&rs).await;
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
            }

            _ = send_stream.close().await;
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
