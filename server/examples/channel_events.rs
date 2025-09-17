use std::{path::Path, time::Duration};

use common::structs::channel::{ChannelEvent, ChannelEvents};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use anyhow::anyhow;
use clap::Parser;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Certificate, Client, Identity,
};

#[derive(Parser, Debug)]
#[command(version, about = "Channel Events Tester - Manual API testing tool", long_about = None)]
struct Opt {
    /// The CA Certificate path
    #[arg(long, default_value = "./examples/test_certs/ca.crt")]
    ca_cert: String,

    /// The certificate file
    #[arg(short, long, default_value = "./examples/test_certs/test.crt")]
    cert: String,

    /// The key file
    #[arg(short, long, default_value = "./examples/test_certs/test.key")]
    key: String,

    /// The endpoint base URL
    #[arg(short, long, default_value = "https://local.bedrockvc.stream")]
    uri: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Create a new channel
    Create {
        /// Channel name to create
        #[arg(short, long)]
        name: String,
    },
    /// Delete a channel by ID
    Delete {
        /// Channel ID to delete
        #[arg(short, long)]
        id: String,
    },
    /// Join a channel by ID
    Join {
        /// Channel ID to join
        #[arg(short, long)]
        id: String,
    },
    /// Leave a channel by ID
    Leave {
        /// Channel ID to leave
        #[arg(short, long)]
        id: String,
    },
    /// List all channels
    List,
}

impl Opt {
    pub async fn get_ca_bytes(&self) -> Result<Certificate, anyhow::Error> {
        let mut buf = Vec::new();
        File::open(Path::new(&self.ca_cert))
            .await
            .map_err(|e| anyhow!("Failed to open CA cert file: {}", e))?
            .read_to_end(&mut buf)
            .await
            .map_err(|e| anyhow!("Failed to read CA cert file: {}", e))?;

        match reqwest::Certificate::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!("Failed to parse CA cert: {}", e)),
        }
    }

    pub async fn get_identity_bytes(&self) -> Result<Identity, anyhow::Error> {
        // Read certificate file
        let cert_content = tokio::fs::read_to_string(&self.cert)
            .await
            .map_err(|e| anyhow!("Failed to read cert file {}: {}", &self.cert, e))?;

        // Read key file
        let key_content = tokio::fs::read_to_string(&self.key)
            .await
            .map_err(|e| anyhow!("Failed to read key file {}: {}", &self.key, e))?;

        // Combine cert and key into proper PEM format
        let combined_pem = format!("{}\n{}", cert_content.trim(), key_content.trim());
        println!("Combined PEM length: {} bytes", combined_pem.len());

        match reqwest::Identity::from_pem(combined_pem.as_bytes()) {
            Ok(identity) => Ok(identity),
            Err(e) => Err(anyhow!("Failed to create identity from PEM: {}", e)),
        }
    }

    pub async fn get_reqwest_client(&self) -> Result<Client, anyhow::Error> {
        let ca_cert = self.get_ca_bytes().await?;
        let identity = self.get_identity_bytes().await?;
        
        let mut builder = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::new(10, 0))
            .danger_accept_invalid_certs(false)
            .add_root_certificate(ca_cert)
            .identity(identity);

        builder = builder.danger_accept_invalid_certs(true);

        match builder.build() {
            Ok(client) => Ok(client),
            Err(e) => Err(anyhow!("Failed to build HTTP client: {}", e)),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    let client = opt.get_reqwest_client().await?;
    
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));

    match opt.command {
        Commands::Create { name } => {
            println!("🆕 Creating channel '{}'...", name);
            let url = format!("{}/api/channel", opt.uri);
            match client.post(&url).headers(headers).json(&name).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let channel_id: String = response.json().await?;
                        println!("✅ Channel created successfully!");
                        println!("📋 Channel ID: {}", channel_id);
                    } else {
                        println!("❌ Failed to create channel: {}", response.status());
                        if let Ok(text) = response.text().await {
                            println!("   Response: {}", text);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        
        Commands::Delete { id } => {
            println!("🗑️ Deleting channel '{}'...", id);
            let url = format!("{}/api/channel/{}", opt.uri, id);
            match client.delete(&url).headers(headers).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("✅ Channel deleted successfully!");
                    } else {
                        println!("❌ Failed to delete channel: {}", response.status());
                        if let Ok(text) = response.text().await {
                            println!("   Response: {}", text);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        
        Commands::Join { id } => {
            println!("� Joining channel '{}'...", id);
            let url = format!("{}/api/channel/{}", opt.uri, id);
            let join_event = ChannelEvent::new(ChannelEvents::Join);
            match client.put(&url).headers(headers).json(&join_event).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("✅ Successfully joined channel!");
                    } else {
                        println!("❌ Failed to join channel: {}", response.status());
                        if let Ok(text) = response.text().await {
                            println!("   Response: {}", text);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        
        Commands::Leave { id } => {
            println!("🚪 Leaving channel '{}'...", id);
            let url = format!("{}/api/channel/{}", opt.uri, id);
            let leave_event = ChannelEvent::new(ChannelEvents::Leave);
            match client.put(&url).headers(headers).json(&leave_event).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("✅ Successfully left channel!");
                    } else {
                        println!("❌ Failed to leave channel: {}", response.status());
                        if let Ok(text) = response.text().await {
                            println!("   Response: {}", text);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
        
        Commands::List => {
            println!("📋 Listing all channels...");
            let url = format!("{}/api/channel", opt.uri);
            match client.get(&url).headers(headers).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let channels: serde_json::Value = response.json().await?;
                        println!("✅ Channels retrieved successfully!");
                        println!("{}", serde_json::to_string_pretty(&channels)?);
                    } else {
                        println!("❌ Failed to list channels: {}", response.status());
                        if let Ok(text) = response.text().await {
                            println!("   Response: {}", text);
                        }
                    }
                }
                Err(e) => println!("❌ Error: {}", e),
            }
        }
    }

    Ok(())
}

