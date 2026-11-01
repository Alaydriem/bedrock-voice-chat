//! The WebSocket voice listener presents the certificate on disk at the time of the
//! handshake, not the one it was started with.

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use bvc_server_lib::stream::session::ws::ReloadableCertResolver;
use rcgen::{CertificateParams, KeyPair};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, ServerConfig, ServerConnection};
use tempfile::TempDir;

struct Issued {
    der: Vec<u8>,
}

/// Writes a fresh self-signed certificate and its key to the two paths.
fn write_certificate(certificate: &Path, key: &Path) -> Issued {
    let key_pair = KeyPair::generate().expect("generate a key");
    let params = CertificateParams::new(vec!["bvc.test".to_string()]).expect("certificate params");
    let issued = params.self_signed(&key_pair).expect("self-sign");

    std::fs::write(certificate, issued.pem()).expect("write the certificate");
    std::fs::write(key, key_pair.serialize_pem()).expect("write the key");

    Issued {
        der: issued.der().to_vec(),
    }
}

/// Accepts whatever the server presents: what is under test is which certificate that is.
#[derive(Debug)]
struct AcceptAny;

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::aws_lc_rs::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Completes a handshake against a `ServerConfig` built on the resolver and returns the leaf
/// the client saw.
fn presented_leaf(resolver: Arc<ReloadableCertResolver>) -> Vec<u8> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());

    let server_config = ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("server protocol versions")
        .with_no_client_auth()
        .with_cert_resolver(resolver);

    let client_config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("client protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAny))
        .with_no_client_auth();

    let mut client = rustls::Connection::from(
        ClientConnection::new(
            Arc::new(client_config),
            ServerName::try_from("bvc.test").expect("server name"),
        )
        .expect("client connection"),
    );
    let mut server = rustls::Connection::from(
        ServerConnection::new(Arc::new(server_config)).expect("server connection"),
    );

    for _ in 0..16 {
        if !client.is_handshaking() && !server.is_handshaking() {
            break;
        }
        Pipe::carry(&mut client, &mut server);
        Pipe::carry(&mut server, &mut client);
    }

    assert!(
        !client.is_handshaking(),
        "the handshake did not complete in memory"
    );

    client
        .peer_certificates()
        .and_then(|chain| chain.first())
        .map(|leaf| leaf.as_ref().to_vec())
        .expect("the server presented no certificate")
}

/// Moves the TLS bytes one side has queued to the other.
struct Pipe;

impl Pipe {
    fn carry(from: &mut rustls::Connection, to: &mut rustls::Connection) {
        let mut bytes = Vec::new();
        while from.wants_write() {
            from.write_tls(&mut bytes).expect("write TLS bytes");
        }
        if bytes.is_empty() {
            return;
        }

        let mut cursor = Cursor::new(bytes);
        while (cursor.position() as usize) < cursor.get_ref().len() {
            to.read_tls(&mut cursor).expect("read TLS bytes");
            to.process_new_packets().expect("process TLS packets");
        }
    }
}

#[test]
fn a_reload_serves_the_new_certificate() {
    let dir = TempDir::new().unwrap();
    let certificate = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");

    let a = write_certificate(&certificate, &key);
    let resolver = ReloadableCertResolver::load(
        certificate.to_str().unwrap(),
        key.to_str().unwrap(),
    )
    .expect("load certificate A");
    assert_eq!(presented_leaf(resolver.clone()), a.der);

    let b = write_certificate(&certificate, &key);
    assert_ne!(a.der, b.der, "the two certificates must differ for this test to mean anything");
    resolver.reload().expect("reload certificate B");

    assert_eq!(presented_leaf(resolver), b.der);
}

#[test]
fn a_failed_reload_keeps_serving_the_current_certificate() {
    let dir = TempDir::new().unwrap();
    let certificate = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");

    let a = write_certificate(&certificate, &key);
    let resolver = ReloadableCertResolver::load(
        certificate.to_str().unwrap(),
        key.to_str().unwrap(),
    )
    .expect("load certificate A");

    // A renewal that has replaced the certificate but not yet the key: the pair does not
    // belong together, and the listener must not start presenting it.
    let other = dir.path().join("other-cert.pem");
    let other_key = dir.path().join("other-key.pem");
    write_certificate(&other, &other_key);
    std::fs::copy(&other, &certificate).unwrap();

    assert!(resolver.reload().is_err());
    assert_eq!(presented_leaf(resolver), a.der);
}
