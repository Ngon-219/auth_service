use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add student_code column as nullable
        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .add_column(ColumnDef::new(User::StudentCode).string().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop student_code column
        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .drop_column(User::StudentCode)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    StudentCode,
}
