pub use sea_orm_migration::prelude::*;

mod m20231220_000001_player;
mod m20260119_000001_player_game;
mod m20260307_000001_player_identity;
mod m20260311_000001_player_auth_code;
mod m20260322_000001_audio_file;
mod m20260322_000002_player_permission;
mod m20260618_000001_player_auth_code_ephemeral;
mod m20260726_000001_signed_timestamps;
mod m20260808_000001_player_world;
mod m20260814_000001_drop_peer_link_permission;
mod m20260816_000001_certificate_revocation;
mod m20260817_000001_certificate_authority;
mod m20260819_000001_drop_hytale_rows;
mod m20260823_000001_server_secrets;
mod m20260826_000001_peer_pairing;
mod m20260901_000001_game_access_token;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231220_000001_player::Migration),
            Box::new(m20260119_000001_player_game::Migration),
            Box::new(m20260307_000001_player_identity::Migration),
            Box::new(m20260311_000001_player_auth_code::Migration),
            Box::new(m20260322_000001_audio_file::Migration),
            Box::new(m20260322_000002_player_permission::Migration),
            Box::new(m20260618_000001_player_auth_code_ephemeral::Migration),
            Box::new(m20260726_000001_signed_timestamps::Migration),
            Box::new(m20260808_000001_player_world::Migration),
            Box::new(m20260814_000001_drop_peer_link_permission::Migration),
            Box::new(m20260816_000001_certificate_revocation::Migration),
            Box::new(m20260817_000001_certificate_authority::Migration),
            Box::new(m20260819_000001_drop_hytale_rows::Migration),
            Box::new(m20260823_000001_server_secrets::Migration),
            Box::new(m20260826_000001_peer_pairing::Migration),
            Box::new(m20260901_000001_game_access_token::Migration),
        ]
    }
}
