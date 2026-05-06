use common::Game;

#[derive(Debug, Clone)]
pub struct Identity {
    pub gamertag: String,
    pub game: Game,
    pub server_url: String,
    pub cert_pem: String,
    pub key_pem: String,
    pub ca_pem: String,
    pub cert_not_after: Option<i64>,
}
