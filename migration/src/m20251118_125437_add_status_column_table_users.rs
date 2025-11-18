use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(UserStatus::Table)
                    .values([
                        UserStatus::Pending,
                        UserStatus::Sync,
                        UserStatus::Failed,
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .add_column(
                        ColumnDef::new(User::Status)
                            .enumeration(
                                UserStatus::Table,
                                [
                                    UserStatus::Pending,
                                    UserStatus::Sync,
                                    UserStatus::Failed,
                                ],
                            )
                            .not_null()
                            .default("pending"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(User::Table)
                    .drop_column(User::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_type(Type::drop().name(UserStatus::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Status,
}

#[derive(DeriveIden)]
enum UserStatus {
    Table,
    Pending,
    Sync,
    Failed,
}

