use sea_orm_migration::{prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .add_column(
                        ColumnDef::new(DocumentType::TemplatePdf)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DocumentType::Table)
                    .drop_column(DocumentType::TemplatePdf)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum DocumentType {
    Table,
    TemplatePdf,
}
