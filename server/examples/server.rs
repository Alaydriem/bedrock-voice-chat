#![deny(elided_lifetimes_in_paths)]

use s2n_quic::Server;
use std::{ error::Error, path::Path };
use std::sync::Arc;
use std::fs;
use std::collections::HashMap;
use async_mutex::Mutex;
use serde::{ Serialize, Deserialize };
use rcgen::{
    Certificate,
    CertificateParams,
    DistinguishedName,
    IsCa,
    KeyPair,
    PKCS_ED25519,
    SanType,
    ExtendedKeyUsagePurpose,
    KeyUsagePurpose,
};
use rocket::time::OffsetDateTime;
use std::{ fs::File, io::Write };
use rocket::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packet {
    pub client_id: i32,
    pub data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    generate_ca().await;
    let result = server().await;
    println!("{:?}", result);
}

async fn server() -> Result<(), Box<dyn Error>> {
    // Buffer Setup
    let ring = async_ringbuf::AsyncHeapRb::<Packet>::new(1000);
    let (main_producer, main_consumer) = ring.split();

    let main_producer_mux = Arc::new(Mutex::new(main_producer));
    let main_consumer_mux = Arc::new(Mutex::new(main_consumer));

    let mutex_map = Arc::new(
        Mutex::new(
            HashMap::<
                u64,
                Arc<
                    Mutex<
                        async_ringbuf::AsyncProducer<
                            Packet,
                            Arc<
                                async_ringbuf::AsyncRb<
                                    Packet,
                                    ringbuf::SharedRb<Packet, Vec<std::mem::MaybeUninit<Packet>>>
                                >
                            >
                        >
                    >
                >
            >::new()
        )
    );
    let processor_mutex_map = mutex_map.clone();

    let ca_cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/cert.pem");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/cert.pem");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/key.pem");

    let CA = Path::new(ca_cert_path);
    let CERT = Path::new(cert_path);
    let KEY = Path::new(key_path);

    let provider = MtlsProvider::new(CERT, CERT, KEY).await?;

    let kp = KeyPair::from_pem(&fs::read_to_string(key_path).unwrap()).unwrap();
    let cp = CertificateParams::from_ca_cert_pem(
        &fs::read_to_string(cert_path).unwrap(),
        kp
    ).unwrap();

    let certificate = Certificate::from_params(cp).unwrap();
    signed_cert_with_ca(certificate, "Alaydriem".to_string());
    initialize_logger("server");
    // Initialize the server
    let mut server = Server::builder()
        .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
        .with_tls(provider)?
        .with_io("127.0.0.1:4433")?
        .start()?;

    let mut tasks = Vec::new();
    tasks.push(
        tokio::spawn(async move {
            println!("Started server listening...");
            let mut connection_id: Option<u64> = None;
            while let Some(mut connection) = server.accept().await {
                connection_id = Some(connection.id());
                let main_producer_mux = main_producer_mux.clone();

                // Create the mutex map when the stream is created
                let mut mutex_map = mutex_map.lock_arc().await;

                let ring = async_ringbuf::AsyncHeapRb::<Packet>::new(1000);
                let (producer, consumer) = ring.split();
                let actual_consumer = Arc::new(Mutex::new(consumer));
                mutex_map.insert(connection.id(), Arc::new(Mutex::new(producer)));

                // Each connection sits in it's own thread
                tokio::spawn(async move {
                    let main_producer_mux = main_producer_mux.clone();

                    match connection.accept_bidirectional_stream().await {
                        Ok(stream) =>
                            match stream {
                                Some(stream) => {
                                    let (mut receive_stream, mut send_stream) = stream.split();

                                    let cid: Option<i32> = None;
                                    let client_id = Arc::new(Mutex::new(cid));
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
                                                    if client_id.is_none() {
                                                        *client_id = Some(packet.client_id);
                                                    }

                                                    let mut main_producer_mux =
                                                        main_producer_mux.lock_arc().await;
                                                    _ = main_producer_mux.push(packet).await;
                                                }
                                                Err(_e) => {}
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
                                                    match *client_id {
                                                        Some(client_id) => {
                                                            if client_id.eq(&packet.client_id) {
                                                                continue;
                                                            }
                                                        }
                                                        None => {}
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

            match connection_id {
                Some(connection_id) => {
                    let mut mutex_map = mutex_map.lock().await;
                    mutex_map.remove(&connection_id);
                }
                None => {}
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
                        for (_, producer) in mutex_map.clone().into_iter() {
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

async fn generate_ca() {
    // Create the root CA certificate if it doesn't already exist.
    let root_ca_path_str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/cert.pem");
    let root_ca_key_path_str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/key.pem");
    let root_ca_path = Path::new(&root_ca_path_str);

    if !root_ca_path.exists() {
        let root_kp = match KeyPair::generate(&PKCS_ED25519) {
            Ok(r) => r,
            Err(_) =>
                panic!(
                    "Unable to generate root key. Check the certs_path configuration variable to ensure the path is writable"
                ),
        };

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");

        let mut cp = CertificateParams::new(vec!["127.0.0.1".to_string(), "localhost".to_string()]);

        cp.subject_alt_names = vec![
            SanType::DnsName(String::from("localhost")),
            SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
            SanType::IpAddress(
                std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
            )
        ];
        cp.alg = &PKCS_ED25519;
        cp.is_ca = IsCa::NoCa;
        cp.not_before = OffsetDateTime::now_utc().checked_sub(Duration::days(3)).unwrap();
        cp.distinguished_name = distinguished_name;
        cp.key_pair = Some(root_kp);
        cp.use_authority_key_identifier_extension = true;

        cp.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        cp.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth
        ];
        let root_certificate = match Certificate::from_params(cp) {
            Ok(c) => c,
            Err(_) =>
                panic!(
                    "Unable to generate root certificates. Check the certs_path configuration variable to ensure the path is writable"
                ),
        };

        let cert = root_certificate.serialize_pem_with_signer(&root_certificate).unwrap();
        let key = root_certificate.get_key_pair().serialize_pem();

        let mut key_file = File::create(root_ca_path_str).unwrap();
        key_file.write_all(cert.as_bytes()).unwrap();
        let mut cert_file = File::create(root_ca_key_path_str).unwrap();
        cert_file.write_all(key.as_bytes()).unwrap();
    }
}

fn signed_cert_with_ca(ca_cert: Certificate, dn_name: String) {
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, dn_name.clone());

    let mut params = CertificateParams::default();

    params.distinguished_name = dn;
    params.alg = &PKCS_ED25519;
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth
    ];
    params.not_before = OffsetDateTime::now_utc().checked_sub(Duration::days(3)).unwrap();
    params.not_after = OffsetDateTime::now_utc() + Duration::days(365 * 20);

    params.subject_alt_names = vec![
        SanType::DnsName(dn_name.clone()),
        SanType::DnsName(String::from("localhost")),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
        SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)))
    ];
    let cert = Certificate::from_params(params).unwrap();

    let cert_signed = cert.serialize_pem_with_signer(&ca_cert).unwrap();

    let cert_path = format!("{}/examples/{}.pem", env!("CARGO_MANIFEST_DIR"), dn_name.clone());
    let key_path = format!("{}/examples/{}.key", env!("CARGO_MANIFEST_DIR"), dn_name.clone());
    fs::write(&cert_path, cert_signed).unwrap();
    fs::write(&key_path, &cert.serialize_private_key_pem().as_bytes()).unwrap();
}

use rustls::{ cipher_suite, ClientConfig, RootCertStore, ServerConfig, SupportedCipherSuite };
use s2n_quic::provider::{ tls, tls::rustls::rustls };
use std::{ io::Cursor };
use tokio::{ io::AsyncReadExt };
use tracing::Level;

static PROTOCOL_VERSIONS: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS13];

pub static DEFAULT_CIPHERSUITES: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
];

pub fn initialize_logger(endpoint: &str) {
    use std::sync::Once;

    static TRACING: Once = Once::new();

    // make sure this only gets initialized once (per process)
    TRACING.call_once(|| {
        // always write to the same file, and don't rotate it. This would be a
        // bad idea for a long running process, but is useful to make sure that
        // all the logs of our program end up in the same file.
        let file_appender = tracing_appender::rolling::never("logs", format!("{endpoint}.txt"));

        tracing_subscriber
            ::fmt()
            .with_max_level(Level::DEBUG)
            // don't color the output, otherwise the text logs will have odd
            // characters
            .with_ansi(false)
            .with_writer(file_appender)
            .init();
    });
}

pub struct MtlsProvider {
    root_store: rustls::RootCertStore,
    my_cert_chain: Vec<rustls::Certificate>,
    my_private_key: rustls::PrivateKey,
}

impl tls::Provider for MtlsProvider {
    type Server = tls::rustls::Server;
    type Client = tls::rustls::Client;
    type Error = rustls::Error;

    fn start_server(self) -> Result<Self::Server, Self::Error> {
        let verifier = rustls::server::AllowAnyAuthenticatedClient::new(self.root_store);
        let mut cfg = ServerConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHERSUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(PROTOCOL_VERSIONS)?
            .with_client_cert_verifier(Arc::new(verifier))
            .with_single_cert(self.my_cert_chain, self.my_private_key.clone())?;

        cfg.max_fragment_size = None;
        cfg.alpn_protocols = vec![b"h3".to_vec()];
        Ok(cfg.into())
    }

    fn start_client(self) -> Result<Self::Client, Self::Error> {
        let mut cfg = ClientConfig::builder()
            .with_cipher_suites(DEFAULT_CIPHERSUITES)
            .with_safe_default_kx_groups()
            .with_protocol_versions(PROTOCOL_VERSIONS)?
            .with_root_certificates(self.root_store)
            .with_client_auth_cert(self.my_cert_chain, self.my_private_key)?;

        cfg.max_fragment_size = None;
        cfg.enable_early_data = true;
        cfg.alpn_protocols = vec![b"h3".to_vec()];
        Ok(cfg.into())
    }
}

impl MtlsProvider {
    pub async fn new<A: AsRef<Path>, B: AsRef<Path>, C: AsRef<Path>>(
        ca_cert_pem: A,
        my_cert_pem: B,
        my_key_pem: C
    ) -> Result<Self, rustls::Error> {
        let root_store = into_root_store(ca_cert_pem.as_ref()).await?;
        let cert_chain = into_certificate(my_cert_pem.as_ref()).await?;
        let private_key = into_private_key(my_key_pem.as_ref()).await?;
        Ok(MtlsProvider {
            root_store,
            my_cert_chain: cert_chain.into_iter().map(rustls::Certificate).collect(),
            my_private_key: rustls::PrivateKey(private_key),
        })
    }
}

async fn into_certificate(path: &Path) -> Result<Vec<Vec<u8>>, rustls::Error> {
    let mut f = tokio::fs::File
        ::open(path).await
        .map_err(|e| rustls::Error::General(format!("Failed to load file: {}", e)))?;
    let mut buf = Vec::new();
    f
        .read_to_end(&mut buf).await
        .map_err(|e| rustls::Error::General(format!("Failed to read file: {}", e)))?;
    let mut cursor = Cursor::new(buf);
    let certs = rustls_pemfile
        ::certs(&mut cursor)
        .map(|certs| certs.into_iter().collect())
        .map_err(|_| rustls::Error::General("Could not read certificate".to_string()))?;
    Ok(certs)
}

async fn into_root_store(path: &Path) -> Result<RootCertStore, rustls::Error> {
    let ca_certs = into_certificate(path).await?;
    let mut cert_store = RootCertStore::empty();
    cert_store.add_parsable_certificates(ca_certs.as_slice());
    Ok(cert_store)
}

async fn into_private_key(path: &Path) -> Result<Vec<u8>, rustls::Error> {
    let mut f = tokio::fs::File
        ::open(path).await
        .map_err(|e| rustls::Error::General(format!("Failed to load file: {}", e)))?;
    let mut buf = Vec::new();
    f
        .read_to_end(&mut buf).await
        .map_err(|e| rustls::Error::General(format!("Failed to read file: {}", e)))?;
    let mut cursor = Cursor::new(buf);

    cursor.set_position(0);

    match rustls_pemfile::pkcs8_private_keys(&mut cursor) {
        Ok(keys) if keys.is_empty() => {}
        Ok(mut keys) if keys.len() == 1 => {
            return Ok(rustls::PrivateKey(keys.pop().unwrap()).0);
        }
        Ok(keys) => {
            return Err(
                rustls::Error::General(
                    format!("Unexpected number of keys: {} (only 1 supported)", keys.len())
                )
            );
        }
        // try the next parser
        Err(_) => {}
    }
    Err(rustls::Error::General("could not load any valid private keys".to_string()))
}
