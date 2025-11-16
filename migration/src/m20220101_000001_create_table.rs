use crate::extension::postgres::Type;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create ROLE enum type
        manager
            .create_type(
                Type::create()
                    .as_enum(RoleEnum::Table)
                    .values([
                        RoleEnum::Admin,
                        RoleEnum::Manager,
                        RoleEnum::Teacher,
                        RoleEnum::Student,
                    ])
                    .to_owned(),
            )
            .await?;

        // Create User table
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::UserId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(User::FirstName).string().not_null())
                    .col(ColumnDef::new(User::LastName).string().not_null())
                    .col(ColumnDef::new(User::Address).string().not_null())
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Password).string().not_null())
                    .col(
                        ColumnDef::new(User::IsPriority)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(User::Cccd).string().not_null())
                    .col(ColumnDef::new(User::PhoneNumber).string().not_null())
                    .col(
                        ColumnDef::new(User::IsFirstLogin)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(User::CreateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(User::UpdateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(User::DeletedAt)
                            .timestamp()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(User::Role)
                            .enumeration(
                                RoleEnum::Table,
                                [
                                    RoleEnum::Admin,
                                    RoleEnum::Manager,
                                    RoleEnum::Teacher,
                                    RoleEnum::Student,
                                ],
                            )
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Wallet table
        manager
            .create_table(
                Table::create()
                    .table(Wallet::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Wallet::WalletId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(Wallet::UserId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Wallet::Address).string().not_null())
                    .col(ColumnDef::new(Wallet::PrivateKey).string().not_null())
                    .col(ColumnDef::new(Wallet::ChainType).string().not_null())
                    .col(ColumnDef::new(Wallet::PublicKey).string().not_null())
                    .col(ColumnDef::new(Wallet::Status).string().not_null())
                    .col(ColumnDef::new(Wallet::NetworkId).string().not_null())
                    .col(ColumnDef::new(Wallet::LastUsedAt).timestamp())
                    .col(
                        ColumnDef::new(Wallet::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Wallet::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wallet_user")
                            .from(Wallet::Table, Wallet::UserId)
                            .to(User::Table, User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Major table
        manager
            .create_table(
                Table::create()
                    .table(Major::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Major::MajorId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Major::Name).string().not_null())
                    .col(ColumnDef::new(Major::FoundingDate).timestamp().not_null())
                    .col(
                        ColumnDef::new(Major::CreateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Major::UpdateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Department table
        manager
            .create_table(
                Table::create()
                    .table(Department::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Department::DepartmentId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Department::Name).string().not_null())
                    .col(
                        ColumnDef::new(Department::FoundingDate)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Department::Dean).string().not_null())
                    .col(
                        ColumnDef::new(Department::CreateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Department::UpdateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Major_Department_User junction table
        manager
            .create_table(
                Table::create()
                    .table(MajorDepartmentUser::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MajorDepartmentUser::MajorId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MajorDepartmentUser::DepartmentId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MajorDepartmentUser::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MajorDepartmentUser::CreateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(MajorDepartmentUser::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .primary_key(
                        Index::create()
                            .col(MajorDepartmentUser::MajorId)
                            .col(MajorDepartmentUser::DepartmentId)
                            .col(MajorDepartmentUser::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mdu_major")
                            .from(MajorDepartmentUser::Table, MajorDepartmentUser::MajorId)
                            .to(Major::Table, Major::MajorId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mdu_department")
                            .from(
                                MajorDepartmentUser::Table,
                                MajorDepartmentUser::DepartmentId,
                            )
                            .to(Department::Table, Department::DepartmentId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_mdu_user")
                            .from(MajorDepartmentUser::Table, MajorDepartmentUser::UserId)
                            .to(User::Table, User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(MajorDepartmentUser::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Department::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Major::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Wallet::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        // Drop ROLE enum type
        manager
            .drop_type(Type::drop().name(RoleEnum::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RoleEnum {
    Table,
    Admin,
    Manager,
    Teacher,
    Student,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
    FirstName,
    LastName,
    Address,
    Email,
    Password,
    IsPriority,
    Cccd,
    PhoneNumber,
    IsFirstLogin,
    CreateAt,
    UpdateAt,
    DeletedAt,
    Role,
}

#[derive(DeriveIden)]
enum Wallet {
    Table,
    WalletId,
    UserId,
    Address,
    PrivateKey,
    ChainType,
    PublicKey,
    Status,
    NetworkId,
    LastUsedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Major {
    Table,
    MajorId,
    Name,
    FoundingDate,
    CreateAt,
    UpdateAt,
}

#[derive(DeriveIden)]
enum Department {
    Table,
    DepartmentId,
    Name,
    FoundingDate,
    Dean,
    CreateAt,
    UpdateAt,
}

#[derive(DeriveIden)]
enum MajorDepartmentUser {
    Table,
    MajorId,
    DepartmentId,
    UserId,
    CreateAt,
    UpdatedAt,
}
