//! Mints a client certificate whose Common Name is deliberately NOT a valid issued
//! identity: `minecraft:` — a game tag with no player name, which
//! `ConnectionClassifier` rejects.
//!
//! The certificate is signed by the real dev CA, so it passes the mTLS handshake and
//! is then refused at `accept()` by the identity gate. That makes it the way to
//! exercise the fail-closed path — and the client's terminal `Unauthorized` state —
//! against a real server without forging anything the CA would not have signed.
//!
//! Usage:
//!   cargo run --example mint_refused_cert -- <certs_path> <out_dir>
//!
//! Writes `refused.crt` and `refused.key` into `<out_dir>`.

use std::path::PathBuf;

use bvc_server_lib::services::CertificateService;

fn main() -> Result<(), anyhow::Error> {
    let mut args = std::env::args().skip(1);
    let certs_path = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: mint_refused_cert <certs_path> <out_dir>"))?;
    let out_dir = PathBuf::from(
        args.next()
            .ok_or_else(|| anyhow::anyhow!("usage: mint_refused_cert <certs_path> <out_dir>"))?,
    );

    let service = CertificateService::new(&certs_path)?;
    let (cert, key) = service.sign_player_cert("", &common::Game::Minecraft)?;

    let cert_path = out_dir.join("refused.crt");
    let key_path = out_dir.join("refused.key");
    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key.serialize_pem())?;

    println!("CN=minecraft: (rejected identity)");
    println!("cert: {}", cert_path.display());
    println!("key:  {}", key_path.display());
    Ok(())
}
