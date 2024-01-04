#![deny(elided_lifetimes_in_paths)]

use s2n_quic::Server;
use std::{ error::Error, collections::HashMap };
use std::sync::Arc;
use async_mutex::Mutex;
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
    _ = server().await;
}

async fn server() -> Result<(), Box<dyn Error>> {
    let ring = async_ringbuf::AsyncHeapRb::<Packet>::new(1000);
    let (main_producer, main_consumer) = ring.split();

    let main_producer_mux = Arc::new(Mutex::new(main_producer));
    let main_consumer_mux = Arc::new(Mutex::new(main_consumer));

    // todo!() this is unbound in size as connections get added or are dropped
    let mutex_map = Arc::new(Mutex::new(Vec::new()));
    let processor_mutex_map = mutex_map.clone();

    let mut server = Server::builder()
        .with_tls((CERT_PEM, KEY_PEM))?
        .with_io("127.0.0.1:4433")?
        .start()?;

    let mut tasks = Vec::new();
    tasks.push(
        tokio::spawn(async move {
            while let Some(mut connection) = server.accept().await {
                let main_producer_mux = main_producer_mux.clone();

                // Create the mutex map when the stream is created
                let mut mutex_map = mutex_map.lock_arc().await;

                let ring = async_ringbuf::AsyncHeapRb::<Packet>::new(1000);
                let (producer, consumer) = ring.split();
                let actual_consumer = Arc::new(Mutex::new(consumer));
                mutex_map.push(Arc::new(Mutex::new(producer)));

                // Each connection sits in it's own thread
                tokio::spawn(async move {
                    let main_producer_mux = main_producer_mux.clone();

                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    let client_id = Arc::new(Mutex::new(0_i32));
                                    let rec_client_id = client_id.clone();
                                    let send_client_id = client_id.clone();
                                    // Receiving Stream
                                    tokio::spawn(async move {
                                        let main_producer_mux = main_producer_mux.clone();

                                        while let Ok(Some(data)) = receive_stream.receive().await {
                                            let client_id = rec_client_id.clone();
                                            // Take the data packet, and push it into the main mux
                                            let d = data.clone();
                                            let ds = std::str::from_utf8(&d).unwrap();
                                            match ron::from_str::<Packet>(ds) {
                                                Ok(packet) => {
                                                    let mut client_id = client_id.lock().await;
                                                    if client_id.eq(&0_i32) {
                                                        *client_id = packet.client_id;
                                                    }

                                                    let mut main_producer_mux =
                                                        main_producer_mux.lock_arc().await;
                                                    _ = main_producer_mux.push(packet).await;
                                                }
                                                Err(e) => {}
                                            }
                                        }
                                    });

                                    // Sending Stream
                                    let consumer = actual_consumer.clone();
                                    tokio::spawn(async move {
                                        let consumer = consumer.clone();
                                        let mut consumer = consumer.lock_arc().await;
                                        let client_id = send_client_id.clone();

                                        #[allow(irrefutable_let_patterns)]
                                        while let packet = consumer.pop().await {
                                            match packet {
                                                Some(packet) => {
                                                    let client_id = client_id.lock().await;
                                                    if client_id.eq(&packet.client_id) {
                                                        continue;
                                                    }
                                                    match ron::to_string(&packet) {
                                                        Ok::<String, _>(rs) => {
                                                            let reader = rs.as_bytes().to_vec();

                                                            // Send the data, then flush the buffer
                                                            _ = send_stream.send(
                                                                reader.into()
                                                            ).await;
                                                            _ = send_stream.flush().await;
                                                        }
                                                        Err(_) => {}
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                    });
                                }
                                None => {}
                            }
                        Err(_) => {}
                    }
                });
            }
        })
    );

    // This _really_ wants to be inside of the main connection thread rather than outside
    // We're encountering a deadlock of some kind due to de-refs
    tasks.push(
        tokio::spawn(async move {
            let mut main_consumer_mux = main_consumer_mux.lock_arc().await;

            let mutex_map = processor_mutex_map.clone();
            // Extract the data from the main mux, then push it into everyone elses private mux
            #[allow(irrefutable_let_patterns)]
            while let packet = main_consumer_mux.pop().await {
                match packet {
                    Some(packet) => {
                        let mutex_map = mutex_map.lock_arc().await;
                        for producer in mutex_map.clone().into_iter() {
                            let mut producer = producer.lock_arc().await;

                            // This broadcasts every message to every person
                            // Identify how to eliminate redundant broadcasts
                            _ = producer.push(packet.clone()).await;
                        }
                    }
                    None => {}
                }
            }
            println!("Loop ended");
        })
    );

    for task in tasks {
        _ = task.await;
    }
    Ok(())
}
