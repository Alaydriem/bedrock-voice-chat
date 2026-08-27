pub use sea_orm_migration::prelude::*;

mod m20260824_000001_registry;
mod m20260825_000001_claim;
mod m20260825_000002_storage;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260824_000001_registry::Migration),
            Box::new(m20260825_000001_claim::Migration),
            Box::new(m20260825_000002_storage::Migration),
        ]
    }
}
