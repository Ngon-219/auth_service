use sea_orm_migration::{prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FileUploadHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FileUploadHistory::FileUploadHistoryId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(FileUploadHistory::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FileUploadHistory::FileName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FileUploadHistory::Status)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(FileUploadHistory::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_file_upload_history_user")
                            .from_tbl(FileUploadHistory::Table)
                            .from_col(FileUploadHistory::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_file_upload_history_user_id")
                    .table(FileUploadHistory::Table)
                    .col(FileUploadHistory::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_file_upload_history_user_id")
                    .table(FileUploadHistory::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(FileUploadHistory::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FileUploadHistory {
    Table,
    FileUploadHistoryId,
    UserId,
    FileName,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}
