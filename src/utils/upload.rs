use crate::repositories::file_upload_repository::FileUploadRepository;
use crate::redis_service::redis_service::ChunkUploadProgress;
use axum::Json;
use axum::extract::Multipart;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde_json::json;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub async fn upload_chunk(
    mut multipart: Multipart,
    user_id: &str,
) -> Result<Response, (StatusCode, String)> {
    let mut file_name = String::new();
    let mut chunk_number = 0usize;
    let mut total_chunks = 0usize;
    let mut chunk_data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to read multipart field: {}", e),
        )
    })? {
        let field_name = field.name().unwrap_or_default().to_string();

        match field_name.as_str() {
            "fileName" => {
                file_name = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read fileName field: {}", e),
                    )
                })?;
                file_name = sanitize_filename(&file_name);
            }
            "chunkNumber" => {
                let chunk_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read chunkNumber field: {}", e),
                    )
                })?;
                chunk_number = chunk_str.parse().map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid chunkNumber: {}", e),
                    )
                })?;
            }
            "totalChunks" => {
                let total_str = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read totalChunks field: {}", e),
                    )
                })?;
                total_chunks = total_str.parse().map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid totalChunks: {}", e),
                    )
                })?;
            }
            "chunk" => {
                chunk_data = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Failed to read chunk data: {}", e),
                        )
                    })?
                    .to_vec();
            }
            _ => {}
        }
    }

    // Validate required fields
    if file_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "fileName is required".to_string()));
    }

    if chunk_data.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "chunk data is required".to_string(),
        ));
    }

    if total_chunks == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "totalChunks must be greater than 0".to_string(),
        ));
    }

    // Create temp directory for chunks
    let temp_dir = format!("./uploads/temp/{}", file_name);
    fs::create_dir_all(&temp_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create temp directory: {}", e),
        )
    })?;

    // Save chunk to disk
    let chunk_path = format!("{}/chunk_{}", temp_dir, chunk_number);
    let mut file = fs::File::create(&chunk_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create chunk file: {}", e),
        )
    })?;

    file.write_all(&chunk_data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write chunk data: {}", e),
        )
    })?;

    file.flush().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to flush chunk file: {}", e),
        )
    })?;

    // Create or get file_upload_history_id on first chunk
    let upload_file_history_repo = FileUploadRepository::new();
    let file_upload_history_id = if chunk_number == 0 {
        // First chunk - create file upload history record
        match upload_file_history_repo.create_new_file_upload(&file_name, user_id).await {
            Ok(id) => {
                // Initialize chunk upload progress tracking with file_upload_history_id
                if let Err(e) = ChunkUploadProgress::set_total_chunks(&id.to_string(), total_chunks as u64).await {
                    tracing::warn!("Failed to set total chunks for {}: {}", id, e);
                }
                Some(id)
            }
            Err(e) => {
                tracing::warn!("Failed to create file upload history for {}: {}", file_name, e);
                None
            }
        }
    } else {
        // Try to find existing file upload history by original file_name
        match upload_file_history_repo.find_by_file_name(&file_name).await {
            Ok(Some(record)) => {
                // Initialize if not already set
                if let Err(e) = ChunkUploadProgress::set_total_chunks(&record.file_upload_history_id.to_string(), total_chunks as u64).await {
                    tracing::warn!("Failed to set total chunks for {}: {}", record.file_upload_history_id, e);
                }
                Some(record.file_upload_history_id)
            }
            _ => None
        }
    };

    // Track chunk upload progress using file_upload_history_id if available, otherwise use file_name
    let _progress_key = if let Some(id) = file_upload_history_id {
        if let Err(e) = ChunkUploadProgress::mark_chunk_uploaded(&id.to_string(), chunk_number).await {
            tracing::warn!("Failed to track chunk upload progress for {} chunk {}: {}", id, chunk_number, e);
        }
        id.to_string()
    } else {
        // Fallback to file_name if no history record
        if let Err(e) = ChunkUploadProgress::set_total_chunks(&file_name, total_chunks as u64).await {
            tracing::warn!("Failed to set total chunks for {}: {}", file_name, e);
        }
        if let Err(e) = ChunkUploadProgress::mark_chunk_uploaded(&file_name, chunk_number).await {
            tracing::warn!("Failed to track chunk upload progress for {} chunk {}: {}", file_name, chunk_number, e);
        }
        file_name.clone()
    };

    // Check if all chunks are uploaded
    let is_complete = is_upload_complete(&temp_dir, total_chunks)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check upload status: {}", e),
            )
        })?;

    if is_complete {
        // Assemble final file
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d%H%M%S").to_string();
        let path = std::path::Path::new(&file_name);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let new_file_name = if extension.is_empty() {
            format!("{}_{}", file_stem, timestamp)
        } else {
            format!("{}_{}.{}", file_stem, timestamp, extension)
        };
        let output_path = format!("./uploads/{}", new_file_name);
        if let Err(e) = assemble_file(&temp_dir, &output_path, total_chunks).await {
            // Update status to failed if file assembly fails
            if let Some(id) = file_upload_history_id {
                let _ = upload_file_history_repo
                    .update_status_file_upload(&id.to_string(), crate::repositories::file_upload_repository::FileUploadStatus::Failed)
                    .await;
            }
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to assemble file: {}", e),
            ));
        }

        // Update file_name in existing record or create new one
        if let Some(id) = file_upload_history_id {
            // Update existing record with new file_name (with timestamp)
            if let Err(e) = upload_file_history_repo.update_file_name(&id.to_string(), &new_file_name).await {
                tracing::warn!("Failed to update file_name for {}: {}", id, e);
            }
            // Clean up chunk upload progress tracking
            if let Err(e) = ChunkUploadProgress::reset_progress(&id.to_string()).await {
                tracing::warn!("Failed to reset chunk upload progress for {}: {}", id, e);
            }
        } else {
            // Create new record if not exists
            upload_file_history_repo
                .create_new_file_upload(&new_file_name, user_id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to save upload history: {}", e),
                    )
                })?;
            // Clean up chunk upload progress tracking
            if let Err(e) = ChunkUploadProgress::reset_progress(&file_name).await {
                tracing::warn!("Failed to reset chunk upload progress for {}: {}", file_name, e);
            }
        }

        Ok((
            StatusCode::OK,
            Json(json!({
                "message": "File uploaded successfully",
                "fileName": file_name,
                "chunkNumber": chunk_number,
                "totalChunks": total_chunks,
                "complete": true
            })),
        )
            .into_response())
    } else {
        Ok((
            StatusCode::OK,
            Json(json!({
                "message": "Chunk uploaded successfully",
                "fileName": file_name,
                "chunkNumber": chunk_number,
                "totalChunks": total_chunks,
                "complete": false
            })),
        )
            .into_response())
    }
}

/// Check if all chunks have been uploaded
async fn is_upload_complete(temp_dir: &str, total_chunks: usize) -> Result<bool, std::io::Error> {
    let mut entries = fs::read_dir(temp_dir).await?;
    let mut count = 0usize;

    while let Some(entry) = entries.next_entry().await? {
        let file_name = entry.file_name();
        if file_name.to_string_lossy().starts_with("chunk_") {
            count += 1;
        }
    }

    Ok(count == total_chunks)
}

/// Assemble chunks into final file
async fn assemble_file(
    temp_dir: &str,
    output_path: &str,
    total_chunks: usize,
) -> Result<(), std::io::Error> {
    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).await?;
    }

    // Create output file
    let mut output_file = fs::File::create(output_path).await?;

    // Write all chunks in order
    for chunk_number in 0..total_chunks {
        let chunk_path = format!("{}/chunk_{}", temp_dir, chunk_number);
        let chunk_data = fs::read(&chunk_path).await?;
        output_file.write_all(&chunk_data).await?;
    }

    output_file.flush().await?;

    // Clean up temp directory
    fs::remove_dir_all(temp_dir).await?;

    Ok(())
}
