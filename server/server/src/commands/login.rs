use clap::Parser;
use common::Game;
use tokio::io::AsyncWriteExt;

use crate::commands::admin_api_client::AdminApiClient;
use crate::identity::{Identity, IdentityStore};

const MC_CLIENT_ID: &str = "00000000402b5328";
const MC_REDIRECT_PATH: &str = "/callback";

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Authenticate the CLI against a BVC server via browser OAuth", long_about = None)]
pub struct Config {
    /// Game (minecraft or hytale)
    #[clap(short, long, value_enum, default_value = "minecraft")]
    pub game: Game,
}

impl Config {
    pub async fn run(&self, server_url: &str) {
        let server_url = server_url.trim_end_matches('/').to_string();

        let response = match self.game {
            Game::Minecraft => self.login_minecraft(&server_url).await,
            Game::Hytale => self.login_hytale(&server_url).await,
        };

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Login failed: {}", e);
                std::process::exit(1);
            }
        };

        let identity = Identity {
            gamertag: response.gamertag.clone(),
            game: self.game.clone(),
            server_url: server_url.clone(),
            cert_pem: response.certificate,
            key_pem: response.certificate_key,
            ca_pem: response.certificate_ca,
            cert_not_after: None,
        };

        if let Err(e) = IdentityStore::save(&identity) {
            eprintln!("Failed to persist identity: {}", e);
            std::process::exit(1);
        }

        println!(
            "Logged in as {} ({}) -> identity stored",
            response.gamertag,
            self.game.as_str()
        );
    }

    async fn login_minecraft(
        &self,
        server_url: &str,
    ) -> Result<common::response::LoginResponse, anyhow::Error> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let local_port = listener.local_addr()?.port();
        let redirect_uri = format!("http://127.0.0.1:{}{}", local_port, MC_REDIRECT_PATH);

        let mut oauth_url =
            reqwest::Url::parse("https://login.live.com/oauth20_authorize.srf")?;
        oauth_url
            .query_pairs_mut()
            .append_pair("client_id", MC_CLIENT_ID)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", "XboxLive.signin offline_access");

        println!("Please open the following URL in your browser to authenticate:");
        println!("{}", oauth_url);
        println!();

        let code = Self::capture_redirect_code(listener).await?;

        AdminApiClient::login_minecraft(server_url, None, &code, &redirect_uri)
            .await
            .map_err(|e| anyhow::anyhow!("Server login failed: {}", e))
    }

    async fn login_hytale(
        &self,
        server_url: &str,
    ) -> Result<common::response::LoginResponse, anyhow::Error> {
        let flow = AdminApiClient::start_hytale_device_flow(server_url, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start Hytale device flow: {}", e))?;

        println!("Please open the following URL in your browser to authenticate:");
        println!("{}", flow.verification_uri);
        println!("User code: {}", flow.user_code);
        println!();

        let interval = std::time::Duration::from_secs(flow.interval as u64);
        let expires_in = std::time::Duration::from_secs(flow.expires_in as u64);
        let start = std::time::Instant::now();

        loop {
            tokio::time::sleep(interval).await;

            if start.elapsed() > expires_in {
                return Err(anyhow::anyhow!("Hytale device flow expired"));
            }

            let status = AdminApiClient::poll_hytale_status(server_url, None, &flow.session_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to poll Hytale status: {}", e))?;

            use common::structs::config::HytaleAuthStatus;
            match status.status {
                HytaleAuthStatus::Pending => continue,
                HytaleAuthStatus::Success => {
                    if let Some(login_response) = status.login_response {
                        return Ok(login_response);
                    }
                    return Err(anyhow::anyhow!(
                        "Hytale auth succeeded but no login response returned"
                    ));
                }
                HytaleAuthStatus::Expired => {
                    return Err(anyhow::anyhow!("Hytale device flow expired"));
                }
                HytaleAuthStatus::Denied => {
                    return Err(anyhow::anyhow!("Hytale device flow denied"));
                }
                HytaleAuthStatus::Error => {
                    return Err(anyhow::anyhow!("Hytale device flow error"));
                }
            }
        }
    }

    async fn capture_redirect_code(
        listener: tokio::net::TcpListener,
    ) -> Result<String, anyhow::Error> {
        let (mut stream, _) = listener.accept().await?;

        let mut buf = [0u8; 4096];
        let n = stream.peek(&mut buf).await?;
        if n == 0 {
            return Err(anyhow::anyhow!(
                "Browser closed connection without sending data"
            ));
        }

        let mut read = 0usize;
        loop {
            let n = stream.try_read(&mut buf[read..])?;
            if n == 0 {
                break;
            }
            read += n;
            if read >= buf.len() {
                break;
            }
            if buf[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        let request = String::from_utf8_lossy(&buf[..read]);
        let line = request
            .lines()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty HTTP request"))?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Malformed HTTP request"));
        }
        let path_and_query = parts[1];

        let code = path_and_query
            .split('?')
            .nth(1)
            .and_then(|query| {
                query.split('&').find_map(|pair| {
                    let mut kv = pair.splitn(2, '=');
                    let key = kv.next()?;
                    if key == "code" {
                        kv.next().map(|v| v.to_string())
                    } else {
                        None
                    }
                })
            })
            .ok_or_else(|| anyhow::anyhow!("No 'code' parameter in redirect URL"))?;

        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><body><h1>Authentication successful</h1><p>You can close this tab.</p></body></html>";
        let _ = stream.write_all(response.as_bytes()).await;

        Ok(code)
    }
}
