use crate::extractor::AuthClaims;
use crate::redis_service::redis_service::helper_get_current_file_progress;
use crate::repositories::file_upload_repository::FileUploadRepository;
use crate::utils::upload::upload_chunk;
use axum::{
    Json, Router,
    extract::{Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use utoipa::ToSchema;
use uuid::Uuid;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/upload/chunk", post(upload_file_chunk))
        .route(
            "/api/v1/upload/progress/{file_upload_history_id}",
            get(get_upload_progress),
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

// fn is_csv_extension(filename: &str) -> bool {
//     std::path::Path::new(filename)
//         .extension()
//         .and_then(|ext| ext.to_str())
//         .map(|ext| ext.eq_ignore_ascii_case("csv"))
//         .unwrap_or(false)
// }
