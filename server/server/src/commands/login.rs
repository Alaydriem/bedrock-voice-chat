use clap::Parser;
use common::request::CodeLoginRequest;
use common::Game;

use crate::commands::admin_api_client::AdminApiClient;
use crate::commands::identity::{Identity, IdentityStore};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Authenticate the CLI against a BVC server using a one-time code", long_about = None)]
pub struct Config {
    /// Player gamertag the code was issued for
    #[clap(short = 'p', long)]
    pub gamertag: String,

    /// Game (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// One-time login code (issued by `bvc user generate-code`)
    #[clap(long)]
    pub code: String,
}

impl Config {
    pub async fn run(&self, server_url: &str) {
        let req = CodeLoginRequest {
            gamertag: self.gamertag.clone(),
            code: self.code.clone(),
        };

        let response = match AdminApiClient::login_with_code(server_url, None, &req).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Login failed: {}", e);
                std::process::exit(1);
            }
        };

        let identity = Identity {
            gamertag: response.gamertag.clone(),
            game: self.game.clone(),
            server_url: server_url.trim_end_matches('/').to_string(),
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
}
