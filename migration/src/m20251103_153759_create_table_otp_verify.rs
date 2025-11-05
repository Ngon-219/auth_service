use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OtpVerify::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OtpVerify::OtpId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(OtpVerify::UserId).uuid().not_null())
                    .col(ColumnDef::new(OtpVerify::OtpCode).string().not_null())
                    .col(ColumnDef::new(OtpVerify::Email).string().not_null())
                    .col(ColumnDef::new(OtpVerify::Purpose).string().not_null())
                    .col(
                        ColumnDef::new(OtpVerify::IsVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(OtpVerify::ExpiresAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(OtpVerify::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(OtpVerify::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_otp_verify_user")
                            .from(OtpVerify::Table, OtpVerify::UserId)
                            .to(User::Table, User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_otp_verify_user_id")
                    .table(OtpVerify::Table)
                    .col(OtpVerify::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on email, purpose, and is_verified for querying active OTPs
        manager
            .create_index(
                Index::create()
                    .name("idx_otp_verify_email_purpose")
                    .table(OtpVerify::Table)
                    .col(OtpVerify::Email)
                    .col(OtpVerify::Purpose)
                    .col(OtpVerify::IsVerified)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .name("idx_otp_verify_expires_at")
                    .table(OtpVerify::Table)
                    .col(OtpVerify::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OtpVerify::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum OtpVerify {
    Table,
    OtpId,
    UserId,
    OtpCode,
    Email,
    Purpose,
    IsVerified,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}
