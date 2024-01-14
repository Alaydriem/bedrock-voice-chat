use rustls::{ cipher_suite, ClientConfig, RootCertStore, ServerConfig, SupportedCipherSuite };
use std::io::Cursor;
use tokio::io::AsyncReadExt;
use s2n_quic::provider::{ tls, tls::rustls::rustls };
use std::sync::Arc;
use std::path::Path;

/// What protocols supported by the QUIC stream. TLS 1.3 only
pub(crate) static PROTOCOL_VERSIONS: &[&rustls::SupportedProtocolVersion] = &[
    &rustls::version::TLS13,
];

/// What cipher suite supported by the QUIC Stream
pub(crate) static DEFAULT_CIPHERSUITES: &[SupportedCipherSuite] = &[
    cipher_suite::TLS13_AES_128_GCM_SHA256,
    cipher_suite::TLS13_AES_256_GCM_SHA384,
    cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
];

/// A custom TLS provider for mTLS
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

    pub async fn new_from_string(
        ca_cert_pem: String,
        cert_pem: String,
        key_pem: String
    ) -> Result<Self, rustls::Error> {
        let root_store = into_root_store_bare(ca_cert_pem).await?;
        let cert_chain = into_certificate_bare(cert_pem).await?;
        let private_key = into_private_key_bare(key_pem).await?;
        Ok(MtlsProvider {
            root_store,
            my_cert_chain: cert_chain.into_iter().map(rustls::Certificate).collect(),
            my_private_key: rustls::PrivateKey(private_key),
        })
    }
}

/// Converts a path into a Certificate vec
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

async fn into_certificate_bare(data: String) -> Result<Vec<Vec<u8>>, rustls::Error> {
    let buf = data.as_bytes().to_vec();
    let mut cursor = Cursor::new(buf);
    let certs = rustls_pemfile
        ::certs(&mut cursor)
        .map(|certs| certs.into_iter().collect())
        .map_err(|_| rustls::Error::General("Could not read certificate".to_string()))?;
    Ok(certs)
}

/// Converts a path into a RootCertStore
async fn into_root_store(path: &Path) -> Result<RootCertStore, rustls::Error> {
    let ca_certs = into_certificate(path).await?;
    let mut cert_store = RootCertStore::empty();
    cert_store.add_parsable_certificates(ca_certs.as_slice());
    Ok(cert_store)
}

async fn into_root_store_bare(data: String) -> Result<RootCertStore, rustls::Error> {
    let ca_certs = into_certificate_bare(data).await?;
    let mut cert_store = RootCertStore::empty();
    cert_store.add_parsable_certificates(ca_certs.as_slice());
    Ok(cert_store)
}

/// Converts a certificate key path into a Vec
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

async fn into_private_key_bare(data: String) -> Result<Vec<u8>, rustls::Error> {
    let buf = data.as_bytes().to_vec();
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
