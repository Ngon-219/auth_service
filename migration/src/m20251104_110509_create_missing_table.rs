use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ============================================
        // DOCUMENT SERVICE TABLES
        // ============================================

        // Create document_type table
        manager
            .create_table(
                Table::create()
                    .table(DocumentType::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DocumentType::DocumentTypeId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(DocumentType::DocumentTypeName)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(DocumentType::Description).text().null())
                    .col(
                        ColumnDef::new(DocumentType::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(DocumentType::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create documents table
        manager
            .create_table(
                Table::create()
                    .table(Documents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Documents::DocumentId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Documents::UserId).uuid().not_null())
                    .col(ColumnDef::new(Documents::IssuerId).uuid().not_null())
                    .col(ColumnDef::new(Documents::DocumentTypeId).uuid().not_null())
                    .col(
                        ColumnDef::new(Documents::BlockchainDocId)
                            .string_len(66)
                            .null(),
                    )
                    .col(ColumnDef::new(Documents::TokenId).big_integer().null())
                    .col(ColumnDef::new(Documents::TxHash).string_len(66).null())
                    .col(
                        ColumnDef::new(Documents::ContractAddress)
                            .string_len(42)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Documents::IpfsHash).string().null())
                    .col(ColumnDef::new(Documents::PdfIpfsHash).string().null())
                    .col(
                        ColumnDef::new(Documents::DocumentHash)
                            .string_len(66)
                            .null(),
                    )
                    .col(ColumnDef::new(Documents::Metadata).custom("jsonb").null())
                    .col(
                        ColumnDef::new(Documents::Status)
                            .string()
                            .not_null()
                            .default("draft"),
                    )
                    .col(
                        ColumnDef::new(Documents::IsValid)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Documents::IssuedAt).timestamp().null())
                    .col(ColumnDef::new(Documents::VerifiedAt).timestamp().null())
                    .col(
                        ColumnDef::new(Documents::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Documents::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_documents_document_type")
                            .from_tbl(Documents::Table)
                            .from_col(Documents::DocumentTypeId)
                            .to_tbl(DocumentType::Table)
                            .to_col(DocumentType::DocumentTypeId)
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for documents table
        manager
            .create_index(
                Index::create()
                    .name("idx_documents_user_id")
                    .table(Documents::Table)
                    .col(Documents::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documents_token_id")
                    .table(Documents::Table)
                    .col(Documents::TokenId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documents_blockchain_doc_id")
                    .table(Documents::Table)
                    .col(Documents::BlockchainDocId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documents_pdf_ipfs_hash")
                    .table(Documents::Table)
                    .col(Documents::PdfIpfsHash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_documents_status")
                    .table(Documents::Table)
                    .col(Documents::Status)
                    .col(Documents::IsValid)
                    .to_owned(),
            )
            .await?;

        // ============================================
        // VOTING SERVICE TABLES
        // ============================================

        // Create voting_events table
        manager
            .create_table(
                Table::create()
                    .table(VotingEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VotingEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(
                        ColumnDef::new(VotingEvents::EventId)
                            .big_integer()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(VotingEvents::EventName).string().not_null())
                    .col(ColumnDef::new(VotingEvents::Description).text().not_null())
                    .col(
                        ColumnDef::new(VotingEvents::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VotingEvents::EndTime).timestamp().not_null())
                    .col(
                        ColumnDef::new(VotingEvents::CreatedByAddress)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VotingEvents::CreatedByUserId).uuid().null())
                    .col(
                        ColumnDef::new(VotingEvents::Options)
                            .custom("text[]")
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VotingEvents::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(VotingEvents::TotalVotes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(VotingEvents::TxHash).string().null())
                    .col(ColumnDef::new(VotingEvents::SyncedAt).timestamp().null())
                    .col(
                        ColumnDef::new(VotingEvents::CreatedAtDb)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(VotingEvents::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for voting_events table
        manager
            .create_index(
                Index::create()
                    .name("idx_voting_events_event_id")
                    .table(VotingEvents::Table)
                    .col(VotingEvents::EventId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voting_events_created_by_user_id")
                    .table(VotingEvents::Table)
                    .col(VotingEvents::CreatedByUserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voting_events_is_active")
                    .table(VotingEvents::Table)
                    .col(VotingEvents::IsActive)
                    .to_owned(),
            )
            .await?;

        // Create votes table
        manager
            .create_table(
                Table::create()
                    .table(Votes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Votes::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(Votes::EventId).big_integer().not_null())
                    .col(ColumnDef::new(Votes::VotingEventId).uuid().not_null())
                    .col(ColumnDef::new(Votes::UserAddress).string().not_null())
                    .col(ColumnDef::new(Votes::UserId).uuid().null())
                    .col(ColumnDef::new(Votes::Option).string().not_null())
                    .col(ColumnDef::new(Votes::TxHash).string().null())
                    .col(ColumnDef::new(Votes::BlockNumber).big_integer().null())
                    .col(ColumnDef::new(Votes::VotedAt).timestamp().not_null())
                    .col(
                        ColumnDef::new(Votes::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_votes_voting_event")
                            .from_tbl(Votes::Table)
                            .from_col(Votes::VotingEventId)
                            .to_tbl(VotingEvents::Table)
                            .to_col(VotingEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint for (eventId, userAddress)
        manager
            .create_index(
                Index::create()
                    .name("unique_vote_per_user_per_event")
                    .table(Votes::Table)
                    .col(Votes::EventId)
                    .col(Votes::UserAddress)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create indexes for votes table
        manager
            .create_index(
                Index::create()
                    .name("idx_votes_event_id")
                    .table(Votes::Table)
                    .col(Votes::EventId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_votes_voting_event_id")
                    .table(Votes::Table)
                    .col(Votes::VotingEventId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_votes_user_address")
                    .table(Votes::Table)
                    .col(Votes::UserAddress)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_votes_user_id")
                    .table(Votes::Table)
                    .col(Votes::UserId)
                    .to_owned(),
            )
            .await?;

        // Seed document types
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                INSERT INTO "document_type" ("document_type_name", "description") 
                VALUES 
                    ('Diploma', 'Bằng tốt nghiệp'),
                    ('Transcript', 'Bảng điểm'),
                    ('Certificate', 'Chứng chỉ'),
                    ('Recommendation', 'Thư giới thiệu')
                ON CONFLICT ("document_type_name") DO NOTHING
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_votes_user_id")
                    .table(Votes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_votes_user_address")
                    .table(Votes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_votes_voting_event_id")
                    .table(Votes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_votes_event_id")
                    .table(Votes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("unique_vote_per_user_per_event")
                    .table(Votes::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_voting_events_is_active")
                    .table(VotingEvents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_voting_events_created_by_user_id")
                    .table(VotingEvents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_voting_events_event_id")
                    .table(VotingEvents::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_documents_status")
                    .table(Documents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documents_pdf_ipfs_hash")
                    .table(Documents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documents_blockchain_doc_id")
                    .table(Documents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documents_token_id")
                    .table(Documents::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_documents_user_id")
                    .table(Documents::Table)
                    .to_owned(),
            )
            .await?;

        // Drop tables
        manager
            .drop_table(Table::drop().table(Votes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(VotingEvents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Documents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DocumentType::Table).to_owned())
            .await?;

        Ok(())
    }
}

// ============================================
// DOCUMENT SERVICE ENTITIES
// ============================================

#[derive(DeriveIden)]
enum DocumentType {
    Table,
    DocumentTypeId,
    DocumentTypeName,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Documents {
    Table,
    DocumentId,
    UserId,
    IssuerId,
    DocumentTypeId,
    BlockchainDocId,
    TokenId,
    TxHash,
    ContractAddress,
    IpfsHash,
    PdfIpfsHash,
    DocumentHash,
    Metadata,
    Status,
    IsValid,
    IssuedAt,
    VerifiedAt,
    CreatedAt,
    UpdatedAt,
}

// ============================================
// VOTING SERVICE ENTITIES
// ============================================

#[derive(DeriveIden)]
enum VotingEvents {
    Table,
    Id,
    EventId,
    EventName,
    Description,
    CreatedAt,
    EndTime,
    CreatedByAddress,
    CreatedByUserId,
    Options,
    IsActive,
    TotalVotes,
    TxHash,
    SyncedAt,
    CreatedAtDb,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Votes {
    Table,
    Id,
    EventId,
    VotingEventId,
    UserAddress,
    UserId,
    Option,
    TxHash,
    BlockNumber,
    VotedAt,
    CreatedAt,
}
