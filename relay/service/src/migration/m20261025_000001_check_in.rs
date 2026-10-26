use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Registration {
    Table,
    RevokedAt,
    DeclaredPort,
    CheckinNonce,
    CheckedInAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // sqlite accepts one ADD COLUMN per ALTER TABLE.
        manager
            .alter_table(
                Table::alter()
                    .table(Registration::Table)
                    .add_column(ColumnDef::new(Registration::RevokedAt).big_integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Registration::Table)
                    .add_column(ColumnDef::new(Registration::DeclaredPort).integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Registration::Table)
                    .add_column(ColumnDef::new(Registration::CheckinNonce).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Registration::Table)
                    .add_column(
                        ColumnDef::new(Registration::CheckedInAt)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE registration SET state = 'revoked', revoked_at = suspended_at, \
                 suspended_at = NULL WHERE state = 'suspended'",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE registration SET state = 'suspended', suspended_at = revoked_at \
                 WHERE state = 'revoked'",
            )
            .await?;

        for column in [
            Registration::CheckedInAt,
            Registration::CheckinNonce,
            Registration::DeclaredPort,
            Registration::RevokedAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Registration::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
