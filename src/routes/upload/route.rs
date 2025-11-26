use crate::extractor::AuthClaims;
use crate::redis_service::redis_service::{helper_get_current_file_progress, ChunkUploadProgress};
use crate::repositories::file_upload_repository::FileUploadRepository;
use crate::utils::upload::upload_chunk;
use axum::{
    Json, Router,
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/upload/chunk", post(upload_file_chunk))
        .route(
            "/api/v1/upload/progress/{file_upload_history_id}",
            get(get_upload_progress),
        )
        .route("/api/v1/upload/history", get(get_upload_history))
        .route("/api/v1/upload/chunk-progress", get(get_chunk_upload_progress))
        .route(
            "/api/v1/upload/chunk-progress/{file_upload_history_id}",
            get(get_chunk_upload_progress_by_id),
        )
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadChunkResponse {
    #[schema(example = "Chunk uploaded successfully")]
    pub message: String,
    #[schema(example = "data.xlsx")]
    pub file_name: String,
    #[schema(example = 0)]
    pub chunk_number: usize,
    #[schema(example = 10)]
    pub total_chunks: usize,
    #[schema(example = false)]
    pub complete: bool,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgressResponse {
    pub file_upload_history_id: Uuid,
    pub current: u64,
    pub total: u64,
    pub percent: u64,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ChunkUploadProgressQuery {
    #[param(example = "data.xlsx")]
    pub file_name: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChunkUploadProgressResponse {
    pub file_name: String,
    pub current: u64,
    pub total: u64,
    pub percent: u64,
    pub complete: bool,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadQueryParams {
    #[param(example = 1)]
    pub page: Option<u32>,
    #[param(example = 20)]
    pub page_size: Option<u32>,
    #[param(example = "pending")]
    pub status: Option<String>,
    #[param(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub user_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadResponse {
    pub file_upload_history_id: Uuid,
    pub user_id: Uuid,
    pub file_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileUploadListResponse {
    pub file_uploads: Vec<FileUploadResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u64,
}

#[utoipa::path(
    post,
    path = "/api/v1/upload/chunk",
    request_body(content = String, content_type = "multipart/form-data", description = "Multipart form data with fields: fileName (string), chunkNumber (string), totalChunks (string), chunk (binary)"),
    responses(
        (status = 200, description = "Chunk uploaded successfully", body = UploadChunkResponse),
        (status = 400, description = "Bad request - missing required fields or invalid data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Upload"
)]
pub async fn upload_file_chunk(
    AuthClaims(claims): AuthClaims,
    multipart: Multipart,
) -> impl IntoResponse {
    upload_chunk(multipart, &claims.user_id).await
}

#[utoipa::path(
    get,
    path = "/api/v1/upload/progress/{file_upload_history_id}",
    params(
        ("file_upload_history_id" = Uuid, Path, description = "File upload history ID")
    ),
    responses(
        (status = 200, description = "Progress retrieved successfully", body = UploadProgressResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Upload"
)]
pub async fn get_upload_progress(
    AuthClaims(_claims): AuthClaims,
    Path(file_upload_history_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let file_repo = FileUploadRepository::new();
    let file_record = file_repo
        .find_by_id(&file_upload_history_id.to_string())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to find file upload record: {}", e),
            )
        })?;

    let progress = helper_get_current_file_progress(&file_record.file_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch progress: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(UploadProgressResponse {
            file_upload_history_id,
            current: progress.current,
            total: progress.total,
            percent: progress.percent,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/upload/history",
    params(FileUploadQueryParams),
    responses(
        (status = 200, description = "File upload history retrieved successfully", body = FileUploadListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Upload"
)]
pub async fn get_upload_history(
    AuthClaims(_claims): AuthClaims,
    Query(params): Query<FileUploadQueryParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate pagination parameters
    let page = params.page.unwrap_or(1);
    let page = if page == 0 { 1 } else { page };
    let page_size = params.page_size.unwrap_or(20);
    let page_size = if page_size == 0 || page_size > 100 {
        20
    } else {
        page_size
    };

    let file_repo = FileUploadRepository::new();
    let (file_uploads, total) = file_repo
        .find_all_with_pagination(page, page_size, params.user_id, params.status)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get file upload history: {}", e),
            )
        })?;

    let file_upload_responses: Vec<FileUploadResponse> = file_uploads
        .into_iter()
        .map(|f| FileUploadResponse {
            file_upload_history_id: f.file_upload_history_id,
            user_id: f.user_id,
            file_name: f.file_name,
            status: f.status,
            created_at: f.created_at.to_string(),
        })
        .collect();

    let total_pages = (total as f64 / page_size as f64).ceil() as u64;

    Ok((
        StatusCode::OK,
        Json(FileUploadListResponse {
            file_uploads: file_upload_responses,
            total,
            page,
            page_size,
            total_pages,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/upload/chunk-progress",
    params(ChunkUploadProgressQuery),
    responses(
        (status = 200, description = "Chunk upload progress retrieved successfully", body = ChunkUploadProgressResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Upload"
)]
pub async fn get_chunk_upload_progress(
    AuthClaims(_claims): AuthClaims,
    Query(params): Query<ChunkUploadProgressQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let progress = ChunkUploadProgress::get_progress(&params.file_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch chunk upload progress: {}", e),
            )
        })?;

    let complete = progress.total > 0 && progress.current >= progress.total;

    Ok((
        StatusCode::OK,
        Json(ChunkUploadProgressResponse {
            file_name: params.file_name,
            current: progress.current,
            total: progress.total,
            percent: progress.percent,
            complete,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/upload/chunk-progress/{file_upload_history_id}",
    params(
        ("file_upload_history_id" = Uuid, Path, description = "File upload history ID")
    ),
    responses(
        (status = 200, description = "Chunk upload progress retrieved successfully", body = ChunkUploadProgressResponse),
        (status = 404, description = "File upload history not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Upload"
)]
pub async fn get_chunk_upload_progress_by_id(
    AuthClaims(_claims): AuthClaims,
    Path(file_upload_history_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let file_repo = FileUploadRepository::new();
    let file_record = file_repo
        .find_by_id(&file_upload_history_id.to_string())
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to find file upload record: {}", e),
            )
        })?;

    // Get progress using file_upload_history_id (tracked during upload)
    let progress = ChunkUploadProgress::get_progress(&file_upload_history_id.to_string())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch chunk upload progress: {}", e),
            )
        })?;

    let complete = progress.total > 0 && progress.current >= progress.total;

    Ok((
        StatusCode::OK,
        Json(ChunkUploadProgressResponse {
            file_name: file_record.file_name,
            current: progress.current,
            total: progress.total,
            percent: progress.percent,
            complete,
        }),
    ))
}

// fn is_csv_extension(filename: &str) -> bool {
//     std::path::Path::new(filename)
//         .extension()
//         .and_then(|ext| ext.to_str())
//         .map(|ext| ext.eq_ignore_ascii_case("csv"))
//         .unwrap_or(false)
// }
