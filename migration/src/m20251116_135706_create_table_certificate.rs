use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Certificate::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Certificate::CertificateId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Certificate::UserId).uuid().not_null())
                    .col(ColumnDef::new(Certificate::CertificateType).string().not_null())
                    .col(ColumnDef::new(Certificate::IssuedDate).date().not_null())
                    .col(ColumnDef::new(Certificate::ExpiryDate).date().null())
                    .col(ColumnDef::new(Certificate::Description).text().null())
                    .col(ColumnDef::new(Certificate::Metadata).custom("jsonb").null())
                    .col(
                        ColumnDef::new(Certificate::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Certificate::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_certificate_user")
                            .from_tbl(Certificate::Table)
                            .from_col(Certificate::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_certificate_user_id")
                    .table(Certificate::Table)
                    .col(Certificate::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_certificate_type")
                    .table(Certificate::Table)
                    .col(Certificate::CertificateType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_certificate_expiry_date")
                    .table(Certificate::Table)
                    .col(Certificate::ExpiryDate)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_certificate_expiry_date")
                    .table(Certificate::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_certificate_type")
                    .table(Certificate::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_certificate_user_id")
                    .table(Certificate::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Certificate::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Certificate {
    Table,
    CertificateId,
    UserId,
    CertificateType,
    IssuedDate,
    ExpiryDate,
    Description,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}

