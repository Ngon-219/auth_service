use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Drop old major_department_user table
        manager
            .drop_table(Table::drop().table(MajorDepartmentUser::Table).to_owned())
            .await?;

        // 2. Add department_id column to Major table
        manager
            .alter_table(
                Table::alter()
                    .table(Major::Table)
                    .add_column(ColumnDef::new(Major::DepartmentId).uuid().null())
                    .to_owned(),
            )
            .await?;

        // 3. Add foreign key from Major to Department
        manager
            .alter_table(
                Table::alter()
                    .table(Major::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_major_department")
                            .from_tbl(Major::Table)
                            .from_col(Major::DepartmentId)
                            .to_tbl(Department::Table)
                            .to_col(Department::DepartmentId)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Create new user_major junction table (User ↔ Major many-to-many)
        manager
            .create_table(
                Table::create()
                    .table(UserMajor::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UserMajor::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserMajor::MajorId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserMajor::CreateAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(UserMajor::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .primary_key(
                        Index::create()
                            .col(UserMajor::UserId)
                            .col(UserMajor::MajorId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_major_user")
                            .from(UserMajor::Table, UserMajor::UserId)
                            .to(User::Table, User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_major_major")
                            .from(UserMajor::Table, UserMajor::MajorId)
                            .to(Major::Table, Major::MajorId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Reverse operations

        // 1. Drop user_major table
        manager
            .drop_table(Table::drop().table(UserMajor::Table).to_owned())
            .await?;

        // 2. Drop foreign key and department_id column from Major
        manager
            .alter_table(
                Table::alter()
                    .table(Major::Table)
                    .drop_foreign_key(Alias::new("fk_major_department"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Major::Table)
                    .drop_column(Major::DepartmentId)
                    .to_owned(),
            )
            .await?;

        // 3. Recreate old major_department_user table
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
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum Major {
    Table,
    MajorId,
    DepartmentId,
}

#[derive(DeriveIden)]
enum Department {
    Table,
    DepartmentId,
}

#[derive(DeriveIden)]
enum UserMajor {
    Table,
    UserId,
    MajorId,
    CreateAt,
    UpdatedAt,
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
