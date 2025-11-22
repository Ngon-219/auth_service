use std::fs;
use std::path::Path;

use auth_service::entities::document_type;
use auth_service::static_service::get_database_connection;
use auth_service::utils::tracing::init_standard_tracing;
use base64::{engine::general_purpose, Engine as _};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    init_standard_tracing(env!("CARGO_CRATE_NAME"));

    tracing::info!("Initializing template PDF for Certificate document type...");

    let db_connection = get_database_connection().await;

    let template_path = Path::new("src/sample_template/certificate_pdf_template");

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
        let alt_path = Path::new("src/sample_template/certifiate_template.json");
        if alt_path.exists() {
            fs::read_to_string(alt_path)
                .map_err(|e| anyhow::anyhow!("Failed to read template file: {}", e))?
        } else {
            return Err(anyhow::anyhow!(
                "Template file not found. Looking for: {} or {}",
                template_path.display(),
                alt_path.display()
            ));
        }
    };

    tracing::info!("Template file read successfully, size: {} bytes", template_content.len());

    let document_type = document_type::Entity::find()
        .filter(document_type::Column::DocumentTypeName.eq("Certificate"))
        .one(db_connection)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query document_type: {}", e))?;

    match document_type {
        Some(doc_type) => {
            tracing::info!(
                "Found document_type 'Certificate' with ID: {}",
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
                "✅ Successfully updated template_pdf for document_type 'Certificate' (ID: {})",
                updated.document_type_id
            );
            tracing::info!("Template content length: {} characters", template_content.len());
        }
        None => {
            tracing::warn!("Document type 'Certificate' not found in database");
            tracing::info!("Creating new document_type 'Certificate' with template...");

            let new_doc_type = document_type::ActiveModel {
                document_type_name: Set("Certificate".to_string()),
                description: Set(Some("Certificate document type".to_string())),
                template_pdf: Set(Some(template_content)),
                ..Default::default()
            };

            let created = new_doc_type
                .insert(db_connection)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create document_type: {}", e))?;

            tracing::info!(
                "✅ Successfully created document_type 'Certificate' with template (ID: {})",
                created.document_type_id
            );
        }
    }

    Ok(())
}
