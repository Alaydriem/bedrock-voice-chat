use common::structs::packet::PacketOwner;
use common::Coordinate;
use common::structs::packet::QuicNetworkPacket;
use s2n_quic::{ client::Connect, Client };
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use std::time::Duration;
use std::{ fs::File, io::BufReader, path::Path };
use std::{ error::Error, net::SocketAddr };
use rodio::Decoder;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = client(
        args[1].to_string().parse::<String>().unwrap(),
        args[2].to_string().parse::<String>().unwrap(),
        args[3].to_string().parse::<String>().unwrap(),
        args[4].to_string().parse::<String>().unwrap()
    ).await;
    println!("{:?}", result);
}

async fn client(
    id: String,
    source_file: String,
    socket_addr: String,
    server_name: String
) -> Result<(), Box<dyn Error>> {
    _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs
        ::default_provider()
        .install_default();

    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/ca.crt");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/test.crt");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/test_certs/test.key");

    let ca = Path::new(ca_path);
    let cert = Path::new(cert_path);
    let key = Path::new(key_path);

    let provider = common::rustls::MtlsProvider::new(ca, cert, key).await?;

    let client = Client::builder().with_tls(provider)?.with_io("0.0.0.0:0")?.start()?;

    println!("I am client: {}", id);
    let addr: SocketAddr = socket_addr.parse()?;
    let connect = Connect::new(addr).with_server_name(server_name);
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

            let mut total_chunks = 0;
            for chunk in ss.chunks(480) {
                if chunk.len() < 480 {
                    println!("Unexpected chunk end");
                    break;
                }
                total_chunks = total_chunks + 480;
                let s = encoder.encode_vec(chunk, chunk.len() * 4).unwrap();
                let packet = QuicNetworkPacket {
                    owner: PacketOwner {
                        name: id.clone(),
                        client_id: client_id.clone(),
                    },
                    packet_type: common::structs::packet::PacketType::AudioFrame,
                    data: common::structs::packet::QuicNetworkPacketData::AudioFrame(
                        common::structs::packet::AudioFramePacket {
                            length: s.len(),
                            data: s.clone(),
                            sample_rate: 48000,
                            coordinate: Some(Coordinate { x: 5.0, y: 70.0, z: 5.5 }),
                            dimension: Some(common::Dimension::Overworld),
                        }
                    ),
                };

                match packet.to_vec() {
                    Ok(rs) => {
                        let result = send_stream.write_all(&rs).await;
                        if total_chunks % (48000 * 2) == 0 {
                            sleep(Duration::from_millis(550)).await;
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                    }
                }
            }

            let r = send_stream.close().await;
            println!("Close Stream {:?}", r);
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
