use crate::repositories::file_upload_repository::FileUploadRepository;
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
        assemble_file(&temp_dir, &output_path, total_chunks)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to assemble file: {}", e),
                )
            })?;

        let upload_file_history_repo = FileUploadRepository::new();
        upload_file_history_repo
            .create_new_file_upload(&new_file_name, user_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save upload history: {}", e),
                )
            })?;

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
