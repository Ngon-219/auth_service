pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20251028_000002_restructure_major_department;
mod m20251102_172939_create_table_user_mfa;
mod m20251103_153759_create_table_otp_verify;
mod m20251103_154512_update_relation_user_document_type;
mod m20251104_110509_create_missing_table;
mod m20251105_155044_add_student_code;
mod m20251116_134558_create_table_score_board;
mod m20251116_135706_create_table_certificate;
mod m20251117_021133_create_table_request;
mod m20251118_092402_create_table_file_upload;
mod m20251118_125437_add_status_column_table_users;
mod m20251120_142937_alter_table_document_type;
mod m20251122_070153_add_column_pdf_schema_to_docuemnt;
mod m20251209_170606_update_table_certificate;
mod m20251210_145112_update_table_user;
mod m20251211_104058_add_column_expired_at;

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
            Box::new(m20251104_110509_create_missing_table::Migration),
            Box::new(m20251105_155044_add_student_code::Migration),
            Box::new(m20251116_134558_create_table_score_board::Migration),
            Box::new(m20251116_135706_create_table_certificate::Migration),
            Box::new(m20251117_021133_create_table_request::Migration),
            Box::new(m20251118_092402_create_table_file_upload::Migration),
            Box::new(m20251118_125437_add_status_column_table_users::Migration),
            Box::new(m20251120_142937_alter_table_document_type::Migration),
            Box::new(m20251122_070153_add_column_pdf_schema_to_docuemnt::Migration),
            Box::new(m20251209_170606_update_table_certificate::Migration),
            Box::new(m20251210_145112_update_table_user::Migration),
            Box::new(m20251211_104058_add_column_expired_at::Migration),
        ]
    }
}
