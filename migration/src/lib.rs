pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20251028_000002_restructure_major_department;
mod m20251102_172939_create_table_user_mfa;
mod m20251103_153759_create_table_otp_verify;
mod m20251103_154512_update_relation_user_document_type;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20251028_000002_restructure_major_department::Migration),
            Box::new(m20251102_172939_create_table_user_mfa::Migration),
            Box::new(m20251103_153759_create_table_otp_verify::Migration),
            Box::new(m20251103_154512_update_relation_user_document_type::Migration),
        ]
    }
}
