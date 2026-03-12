pub use sea_orm_migration::prelude::*;

mod m20231220_000001_player;
mod m20260119_000001_player_game;
mod m20260311_000001_player_auth_code;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231220_000001_player::Migration),
            Box::new(m20260119_000001_player_game::Migration),
            Box::new(m20260311_000001_player_auth_code::Migration),
        ]
    }
}
