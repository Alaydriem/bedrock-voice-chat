use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;
use crate::structs::config::Keypair;
use crate::structs::permission::ServerPermissions;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LoginResponse {
    pub gamerpic: String,
    pub gamertag: String,
    pub keypair: Keypair,
    pub signature: Keypair,
    pub certificate: String,
    pub certificate_key: String,
    pub certificate_ca: String,
    pub quic_connect_string: String,
    #[serde(default)]
    pub minecraft_username: Option<String>,
    #[serde(default)]
    pub server_permissions: Option<ServerPermissions>,
    // The game the identity belongs to. A code login has no other way to learn it: the
    // client sends only the code, so without this it would have to ask the user which
    // game they are on and then take their word for it over the server's own record.
    //
    // Optional so a client newer than its server still deserializes a login.
    #[serde(default)]
    pub game: Option<Game>,
}

impl LoginResponse {
    pub fn new(
        gamertag: String,
        gamerpic: String,
        keypair: Keypair,
        signature: Keypair,
        certificate: String,
        certificate_key: String,
        certificate_ca: String,
        quic_connect_string: String,
        server_permissions: Option<ServerPermissions>,
        game: Game,
    ) -> Self {
        Self {
            gamertag,
            gamerpic,
            keypair,
            signature,
            certificate,
            certificate_key,
            certificate_ca,
            quic_connect_string,
            minecraft_username: None,
            server_permissions,
            game: Some(game),
        }
    }
}
