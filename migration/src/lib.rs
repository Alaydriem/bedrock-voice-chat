pub use common::sea_orm_migration::prelude::*;

mod m20231220_000001_player;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20231220_000001_player::Migration)]
    }
}
