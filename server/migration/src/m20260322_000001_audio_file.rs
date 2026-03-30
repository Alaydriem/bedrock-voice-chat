use sea_orm_migration::prelude::*;

use crate::m20231220_000001_player::Player;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AudioFile::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AudioFile::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(AudioFile::UploaderId).integer().not_null())
                    .col(ColumnDef::new(AudioFile::OriginalFilename).string().not_null())
                    .col(ColumnDef::new(AudioFile::DurationMs).big_integer().not_null())
                    .col(ColumnDef::new(AudioFile::FileSizeBytes).big_integer().not_null())
                    .col(
                        ColumnDef::new(AudioFile::Game)
                            .string()
                            .not_null()
                            .default("minecraft"),
                    )
                    .col(ColumnDef::new(AudioFile::Deleted).integer().not_null().default(0))
                    .col(ColumnDef::new(AudioFile::CreatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_audio_file_uploader")
                            .from(AudioFile::Table, AudioFile::UploaderId)
                            .to(Player::Table, Player::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audio_file_uploader")
                    .table(AudioFile::Table)
                    .col(AudioFile::UploaderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audio_file_game")
                    .table(AudioFile::Table)
                    .col(AudioFile::Game)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AudioFile::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum AudioFile {
    Table,
    Id,
    UploaderId,
    OriginalFilename,
    DurationMs,
    FileSizeBytes,
    Game,
    Deleted,
    CreatedAt,
}
