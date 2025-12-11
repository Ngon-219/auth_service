use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop unique constraint on email column
        // PostgreSQL creates unique constraint with name pattern: {table}_{column}_key
        // Use raw SQL to drop the constraint
        let db = manager.get_connection();
        sea_orm::ConnectionTrait::execute(
            db,
            sea_orm::Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE \"user\" DROP CONSTRAINT IF EXISTS user_email_key;".to_string(),
            ),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreate unique constraint on email column
        let db = manager.get_connection();
        sea_orm::ConnectionTrait::execute(
            db,
            sea_orm::Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE \"user\" ADD CONSTRAINT user_email_key UNIQUE (email);".to_string(),
            ),
        )
        .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Email,
}
