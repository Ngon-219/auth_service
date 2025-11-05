use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add created_by column to document_type table
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .add_column(ColumnDef::new(DocumentType::CreatedBy).uuid().null())
                    .to_owned(),
            )
            .await?;

        // Add foreign key from document_type.created_by to user.user_id
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_document_type_created_by")
                            .from_tbl(DocumentType::Table)
                            .from_col(DocumentType::CreatedBy)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on created_by for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_document_type_created_by")
                    .table(DocumentType::Table)
                    .col(DocumentType::CreatedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_document_type_created_by")
                    .table(DocumentType::Table)
                    .to_owned(),
            )
            .await?;

        // Drop foreign key
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .drop_foreign_key(Alias::new("fk_document_type_created_by"))
                    .to_owned(),
            )
            .await?;

        // Drop created_by column
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .drop_column(DocumentType::CreatedBy)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum DocumentType {
    Table,
    CreatedBy,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}
