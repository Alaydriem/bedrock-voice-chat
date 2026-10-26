pub use sea_orm_migration::prelude::*;

mod m20260824_000001_registry;
mod m20260825_000001_claim;
mod m20260825_000002_storage;
mod m20261025_000001_check_in;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260824_000001_registry::Migration),
            Box::new(m20260825_000001_claim::Migration),
            Box::new(m20260825_000002_storage::Migration),
            Box::new(m20261025_000001_check_in::Migration),
        ]
    }
}
