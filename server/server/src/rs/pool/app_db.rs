use std::ops::{Deref, DerefMut};

use super::SeaOrmPool;

#[derive(Debug)]
pub struct AppDb(SeaOrmPool);

impl From<SeaOrmPool> for AppDb {
    fn from(pool: SeaOrmPool) -> Self {
        AppDb(pool)
    }
}

impl Deref for AppDb {
    type Target = SeaOrmPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl sea_orm_rocket::Database for AppDb {
    const NAME: &'static str = "app";
    type Pool = SeaOrmPool;
}
