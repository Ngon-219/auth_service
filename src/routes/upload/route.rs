use axum::{extract::Multipart, response::IntoResponse, routing::post, Router};
use utoipa::ToSchema;

use crate::utils::upload::upload_chunk;

pub fn create_route() -> Router {
    Router::new().route("/api/v1/upload/chunk", post(upload_file_chunk))
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
pub async fn upload_file_chunk(multipart: Multipart) -> impl IntoResponse {
    upload_chunk(multipart).await
}

