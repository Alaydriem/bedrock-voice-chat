use std::{ error::Error, net::SocketAddr };
use s2n_quic::{ client::Connect, Client };
use serde::{ Serialize, Deserialize };
pub static CERT_PEM: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/cert.pem"));

pub static KEY_PEM: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/key.pem"));

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packet {
    pub client_id: i32,
    pub data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    _ = client(args[1].to_string().parse::<i32>().unwrap()).await;
}

async fn client(id: i32) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().with_tls(CERT_PEM)?.with_io("0.0.0.0:0")?.start()?;

    println!("I am client: {}", id);
    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
    let mut connection = client.connect(connect).await?;

    // ensure the connection doesn't time out with inactivity
    connection.keep_alive(true)?;

    // open a new stream and split the receiving and sending sides
    let stream = connection.open_bidirectional_stream().await?;
    let (mut receive_stream, mut send_stream) = stream.split();

    let mut tasks = Vec::new();
    // spawn a task that copies responses from the server to stdout
    tasks.push(
        tokio::spawn(async move {
            while let Ok(Some(stream)) = receive_stream.receive().await {
                println!("Waiting for data");
                let ds = std::str::from_utf8(&stream).unwrap();
                println!("Got response {}", ds);
                match ron::from_str::<Packet>(ds) {
                    Ok::<Packet, _>(packet) => {
                        println!("Received Data from {}", packet.client_id);
                    }
                    Err(e) => {}
                }
            }

            println!("Recieving loop died");
        })
    );

    tasks.push(
        tokio::spawn(async move {
            loop {
                let data: Vec<u8> = (0..32).map(|_| { rand::random::<u8>() }).collect();
                let packet = Packet { client_id: id, data: data.clone() };
                let rs = ron::to_string(&packet).unwrap();
                let reader = rs.as_bytes().to_vec();
                _ = send_stream.send(reader.into()).await;
                _ = send_stream.flush().await;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            println!("Sending loop died.");
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
}
