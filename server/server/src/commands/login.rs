use clap::Parser;
use common::Game;
use common::request::CodeLoginRequest;

use crate::commands::admin_api_client::AdminApiClient;
use crate::identity::{Identity, IdentityStore};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Authenticate the CLI against a BVC server using a one-time code", long_about = None)]
pub struct Config {
    /// Gamertag the code was issued for
    #[clap(short = 'p', long, env = "BVC_GAMERTAG")]
    pub gamertag: String,

    /// One-time login code (will be prompted on stdin if omitted)
    #[clap(long)]
    pub code: Option<String>,

    /// Game (minecraft or hytale)
    #[clap(short, long, value_enum, default_value = "minecraft")]
    pub game: Game,
}

impl Config {
    pub async fn run(&self, server_url: &str) {
        let server_url = server_url.trim_end_matches('/').to_string();

        let code = match &self.code {
            Some(c) => c.trim().to_string(),
            None => match Self::prompt_code() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read code from stdin: {}", e);
                    std::process::exit(1);
                }
            },
        };

        if code.is_empty() {
            eprintln!("No code provided");
            std::process::exit(1);
        }

        let request = CodeLoginRequest {
            gamertag: self.gamertag.clone(),
            code,
        };

        let response = match AdminApiClient::login_with_code(&server_url, None, &request).await {
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

    fn prompt_code() -> Result<String, std::io::Error> {
        use std::io::Write;
        print!("Code: ");
        std::io::stdout().flush()?;
        let mut code = String::new();
        std::io::stdin().read_line(&mut code)?;
        Ok(code.trim().to_string())
    }
}
