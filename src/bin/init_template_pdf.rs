use std::fs;
use std::path::Path;

use auth_service::entities::document_type;
use auth_service::static_service::get_database_connection;
use auth_service::utils::tracing::init_standard_tracing;
use base64::{engine::general_purpose, Engine as _};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

async fn init_template_for_document_type(
    db_connection: &sea_orm::DatabaseConnection,
    document_type_name: &str,
    template_path: &Path,
    description: &str,
) -> anyhow::Result<()> {
    tracing::info!("Initializing template PDF for {} document type...", document_type_name);

    let template_content = if template_path.exists() {
        match fs::read_to_string(template_path) {
            Ok(content) => content,
            Err(_) => {
                let bytes = fs::read(template_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read template file as binary: {}", e))?;
                general_purpose::STANDARD.encode(&bytes)
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "Template file not found. Looking for: {}",
            template_path.display()
        ));
    };

    tracing::info!("Template file read successfully, size: {} bytes", template_content.len());

    let document_type = document_type::Entity::find()
        .filter(document_type::Column::DocumentTypeName.eq(document_type_name))
        .one(db_connection)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query document_type: {}", e))?;

    match document_type {
        Some(doc_type) => {
            tracing::info!(
                "Found document_type '{}' with ID: {}",
                document_type_name,
                doc_type.document_type_id
            );

            // Update template_pdf field
            let mut active_model: document_type::ActiveModel = doc_type.into();
            active_model.template_pdf = Set(Some(template_content.clone()));

            let updated = active_model
                .update(db_connection)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to update document_type: {}", e))?;

            tracing::info!(
                "✅ Successfully updated template_pdf for document_type '{}' (ID: {})",
                document_type_name,
                updated.document_type_id
            );
            tracing::info!("Template content length: {} characters", template_content.len());
        }
        None => {
            tracing::warn!("Document type '{}' not found in database", document_type_name);
            tracing::info!("Creating new document_type '{}' with template...", document_type_name);

            let new_doc_type = document_type::ActiveModel {
                document_type_name: Set(document_type_name.to_string()),
                description: Set(Some(description.to_string())),
                template_pdf: Set(Some(template_content)),
                ..Default::default()
            };

            let created = new_doc_type
                .insert(db_connection)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create document_type: {}", e))?;

            tracing::info!(
                "✅ Successfully created document_type '{}' with template (ID: {})",
                document_type_name,
                created.document_type_id
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    init_standard_tracing(env!("CARGO_CRATE_NAME"));

    tracing::info!("Initializing templates for Certificate and Diploma document types...");

    let db_connection = get_database_connection().await;

    // Init Certificate
    let certificate_path = Path::new("src/sample_template/certifiate_template.json");
    init_template_for_document_type(
        &db_connection,
        "Certificate",
        certificate_path,
        "Chứng chỉ",
    )
    .await?;

    // Init Diploma
    let diploma_path = Path::new("src/sample_template/diploma_pdf_template.json");
    init_template_for_document_type(
        &db_connection,
        "Diploma",
        diploma_path,
        "Bằng tốt nghiệp",
    )
    .await?;

    tracing::info!("✅ Successfully initialized templates for both Certificate and Diploma!");

    Ok(())
}
