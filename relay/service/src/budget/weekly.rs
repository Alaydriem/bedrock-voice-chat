use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter,
};

use crate::entity::issuance_log;

// How many first issuances the certificate authority will still accept this week.
//
// The ceiling is on the registered domain, so every assigned name draws on one
// budget. The relay is the challenge solver on every issuance, which makes it the
// throttle point without it having to issue anything itself.
pub struct WeeklyBudget {
    conn: Arc<DatabaseConnection>,
    ceiling: u32,
}

impl WeeklyBudget {
    const WINDOW_SECONDS: i64 = 7 * 24 * 60 * 60;

    pub fn new(conn: Arc<DatabaseConnection>, ceiling: u32) -> Self {
        Self { conn, ceiling }
    }

    pub fn new_shared(conn: Arc<DatabaseConnection>, ceiling: u32) -> Arc<Self> {
        Arc::new(Self::new(conn, ceiling))
    }

    pub async fn remaining(&self) -> Result<u32, DbErr> {
        let since = Self::now() - Self::WINDOW_SECONDS;

        let spent = issuance_log::Entity::find()
            .filter(issuance_log::Column::IsRenewal.eq(false))
            .filter(issuance_log::Column::IssuedAt.gte(since))
            .count(self.conn.as_ref())
            .await? as u32;

        Ok(self.ceiling.saturating_sub(spent))
    }

    pub async fn admits_new_issuance(&self) -> Result<bool, DbErr> {
        Ok(self.remaining().await? > 0)
    }

    // Whether this name has ever had a certificate issued. A name that has is
    // renewing, and a renewal is the exempt category at the certificate authority.
    pub async fn has_issued(&self, name: &str) -> Result<bool, DbErr> {
        let count = issuance_log::Entity::find()
            .filter(issuance_log::Column::Name.eq(name))
            .count(self.conn.as_ref())
            .await?;

        Ok(count > 0)
    }

    pub async fn record(&self, name: &str, is_renewal: bool) -> Result<(), DbErr> {
        issuance_log::ActiveModel {
            id: ActiveValue::NotSet,
            name: ActiveValue::Set(name.to_string()),
            is_renewal: ActiveValue::Set(is_renewal),
            issued_at: ActiveValue::Set(Self::now()),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
