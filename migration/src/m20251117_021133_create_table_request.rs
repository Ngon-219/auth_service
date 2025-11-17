use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create REQUEST_STATUS enum type
        manager
            .create_type(
                Type::create()
                    .as_enum(RequestStatus::Table)
                    .values([
                        RequestStatus::Pending,
                        RequestStatus::Scheduled,
                        RequestStatus::Rejected,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create Request table
        manager
            .create_table(
                Table::create()
                    .table(Request::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Request::RequestId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Request::UserId).uuid().not_null())
                    .col(ColumnDef::new(Request::Content).text().not_null())
                    .col(
                        ColumnDef::new(Request::Status)
                            .enumeration(
                                RequestStatus::Table,
                                [
                                    RequestStatus::Pending,
                                    RequestStatus::Scheduled,
                                    RequestStatus::Rejected,
                                ],
                            )
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(Request::ScheduledAt).timestamp().null())
                    .col(
                        ColumnDef::new(Request::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Request::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_request_user")
                            .from_tbl(Request::Table)
                            .from_col(Request::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_request_user_id")
                    .table(Request::Table)
                    .col(Request::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_request_status")
                    .table(Request::Table)
                    .col(Request::Status)
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
                    .name("idx_request_status")
                    .table(Request::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_request_user_id")
                    .table(Request::Table)
                    .to_owned(),
            )
            .await?;

        // Drop table
        manager
            .drop_table(Table::drop().table(Request::Table).to_owned())
            .await?;

        // Drop enum type
        manager
            .drop_type(Type::drop().name(RequestStatus::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Request {
    Table,
    RequestId,
    UserId,
    Content,
    Status,
    ScheduledAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum RequestStatus {
    Table,
    Pending,
    Scheduled,
    Rejected,
}
