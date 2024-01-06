use std::{ error::Error, net::SocketAddr };
use s2n_quic::{ client::Connect, Client };
use serde::{ Serialize, Deserialize };
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packet {
    pub client_id: i32,
    pub data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    initialize_logger("client");
    let args: Vec<String> = std::env::args().collect();
    let result = client(args[1].to_string().parse::<i32>().unwrap()).await;
}

async fn client(id: i32) -> Result<(), Box<dyn Error>> {
    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/cert.pem");
    let cert_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/Alaydriem.pem");
    let key_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/Alaydriem.key");

    let CA = Path::new(ca_path);
    let CERT = Path::new(cert_path);
    let KEY = Path::new(key_path);

    let provider = MtlsProvider::new(CA, CERT, KEY).await?;

    let client = Client::builder().with_tls(provider)?.with_io("0.0.0.0:0")?.start()?;

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
                let ds = std::str::from_utf8(&stream).unwrap();
                match ron::from_str::<Packet>(ds) {
                    Ok::<Packet, _>(packet) => {
                        println!("Received Data from {}", packet.client_id);
                    }
                    Err(_e) => {}
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
        })
    );

    for task in tasks {
        _ = task.await;
    }

    Ok(())
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

#[derive(Debug, Clone)]
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
