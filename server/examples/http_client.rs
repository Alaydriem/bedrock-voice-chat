use std::{ path::Path, time::Duration };

use common::{ structs::channel::{ ChannelEvent, ChannelEvents } };
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use clap::Parser;
use reqwest::{ header::{ HeaderMap, HeaderValue }, Certificate, Client, Identity };
use anyhow::anyhow;

#[derive(Parser, Debug)]
#[command(version, about = "mTLS Client", long_about = None)]
struct Opt {
    /// The CA Certificate path
    #[arg(short, long)]
    ca_cert: String,

    /// The pem
    #[arg(short, long)]
    pem: String,

    /// The endpoint to hit
    #[arg(short, long)]
    uri: String,

    /// The http method
    #[arg(short, long)]
    method: String,

    /// The data to submit
    #[arg(short, long)]
    data: Option<String>,
}

impl Opt {
    pub async fn get_ca_bytes(&self) -> Result<Certificate, anyhow::Error> {
        let mut buf = Vec::new();
        File::open(Path::new(&self.ca_cert)).await.unwrap().read_to_end(&mut buf).await.unwrap();

        match reqwest::Certificate::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    pub async fn get_identity_bytes(&self) -> Result<Identity, anyhow::Error> {
        let mut buf = Vec::new();
        File::open(Path::new(&self.pem)).await.unwrap().read_to_end(&mut buf).await.unwrap();

        match reqwest::Identity::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    pub async fn get_reqwest_client(&self) -> Client {
        let mut builder = reqwest::Client
            ::builder()
            .use_rustls_tls()
            .timeout(Duration::new(5, 0))
            .danger_accept_invalid_certs(false)
            .add_root_certificate(self.get_ca_bytes().await.unwrap())
            .identity(self.get_identity_bytes().await.unwrap());

        builder = builder.danger_accept_invalid_certs(true);

        return builder.build().unwrap();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    let client = opt.get_reqwest_client().await;

    let base_uri = format!("https://127.0.0.1:3000{}", opt.uri);
    let mut c = match opt.method.to_lowercase().as_str() {
        "post" => client.post(base_uri),
        "delete" => client.delete(base_uri),
        "put" => client.put(base_uri),
        _ => client.get(base_uri),
    };

    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));

    c = c.headers(headers);
    match opt.data.clone() {
        Some(data) => {
            c = c.json(&(ChannelEvent { event: ChannelEvents::Leave }));
        }
        None => {}
    }

    match c.send().await {
        Ok(response) => {
            dbg!("{:?}", response.json::<serde_json::Value>().await);
        }
        Err(e) => println!("{}", e.to_string()),
    }

    Ok(())
}
