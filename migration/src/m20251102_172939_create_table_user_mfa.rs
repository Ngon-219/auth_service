use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserMfa::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserMfa::MfaId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(UserMfa::UserId)
                            .uuid()
                            .not_null()
                            .unique_key(), // One-to-One relationship
                    )
                    .col(ColumnDef::new(UserMfa::Secret).string().not_null())
                    .col(
                        ColumnDef::new(UserMfa::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(UserMfa::BackupCodes).string().null())
                    .col(
                        ColumnDef::new(UserMfa::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(UserMfa::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_mfa_user")
                            .from(UserMfa::Table, UserMfa::UserId)
                            .to(User::Table, User::UserId)
                            .on_delete(ForeignKeyAction::Cascade) // Xóa MFA khi xóa user
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_user_mfa_user_id")
                    .table(UserMfa::Table)
                    .col(UserMfa::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserMfa::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserMfa {
    Table,
    MfaId,
    UserId,
    Secret,
    IsEnabled,
    BackupCodes,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}
