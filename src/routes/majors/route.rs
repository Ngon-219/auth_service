use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use uuid::Uuid;

use super::dto::{CreateMajorRequest, MajorListResponse, MajorResponse, UpdateMajorRequest};
use crate::extractor::AuthClaims;
use crate::repositories::{MajorRepository, MajorUpdate};
use do_an_lib::structs::token_claims::UserRole;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/majors", post(create_major))
        .route("/api/v1/majors", get(get_all_majors))
        .route("/api/v1/majors/{major_id}", get(get_major))
        .route("/api/v1/majors/{major_id}", put(update_major))
        .route("/api/v1/majors/{major_id}", delete(delete_major))
}

/// Create a new major (Admin/Manager only)
#[utoipa::path(
    post,
    path = "/api/v1/majors",
    request_body = CreateMajorRequest,
    responses(
        (status = 201, description = "Major created", body = MajorResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden - Admin/Manager only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Majors"
)]
pub async fn create_major(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<CreateMajorRequest>,
) -> Result<(StatusCode, Json<MajorResponse>), (StatusCode, String)> {
    // Check permission: Admin or Manager only
    if auth_claims.role != UserRole::ADMIN && auth_claims.role != UserRole::MANAGER {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin or manager can create majors".to_string(),
        ));
    }
    let major_repo = MajorRepository::new();
    let major_id = Uuid::new_v4();

    let major = major_repo
        .create(
            major_id,
            payload.name,
            payload.founding_date,
            payload.department_id,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create major: {}", e),
            )
        })?;

    let response = MajorResponse {
        major_id: major.major_id,
        name: major.name,
        founding_date: major.founding_date,
        department_id: major.department_id,
        create_at: major.create_at,
        update_at: major.update_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get all majors
#[utoipa::path(
    get,
    path = "/api/v1/majors",
    responses(
        (status = 200, description = "Majors retrieved", body = MajorListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Majors"
)]
pub async fn get_all_majors(
    AuthClaims(_auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<MajorListResponse>), (StatusCode, String)> {
    let major_repo = MajorRepository::new();

    let majors = major_repo.find_all().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get majors: {}", e),
        )
    })?;

    let response = MajorListResponse {
        total: majors.len(),
        majors: majors
            .into_iter()
            .map(|m| MajorResponse {
                major_id: m.major_id,
                name: m.name,
                founding_date: m.founding_date,
                department_id: m.department_id,
                create_at: m.create_at,
                update_at: m.update_at,
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get major by ID (Authenticated users)
#[utoipa::path(
    get,
    path = "/api/v1/majors/{major_id}",
    params(
        ("major_id" = Uuid, Path, description = "Major ID")
    ),
    responses(
        (status = 200, description = "Major retrieved", body = MajorResponse),
        (status = 404, description = "Major not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Majors"
)]
pub async fn get_major(
    AuthClaims(_auth_claims): AuthClaims,
    Path(major_id): Path<Uuid>,
) -> Result<(StatusCode, Json<MajorResponse>), (StatusCode, String)> {
    let major_repo = MajorRepository::new();

    let major = major_repo
        .find_by_id(major_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get major: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Major not found".to_string()))?;

    let response = MajorResponse {
        major_id: major.major_id,
        name: major.name,
        founding_date: major.founding_date,
        department_id: major.department_id,
        create_at: major.create_at,
        update_at: major.update_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Update major (Admin/Manager only)
#[utoipa::path(
    put,
    path = "/api/v1/majors/{major_id}",
    params(
        ("major_id" = Uuid, Path, description = "Major ID")
    ),
    request_body = UpdateMajorRequest,
    responses(
        (status = 200, description = "Major updated", body = MajorResponse),
        (status = 404, description = "Major not found"),
        (status = 403, description = "Forbidden - Admin/Manager only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Majors"
)]
pub async fn update_major(
    AuthClaims(auth_claims): AuthClaims,
    Path(major_id): Path<Uuid>,
    Json(payload): Json<UpdateMajorRequest>,
) -> Result<(StatusCode, Json<MajorResponse>), (StatusCode, String)> {
    // Check permission: Admin or Manager only
    if auth_claims.role != UserRole::ADMIN && auth_claims.role != UserRole::MANAGER {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin or manager can update majors".to_string(),
        ));
    }

    let major_repo = MajorRepository::new();

    let updates = MajorUpdate {
        name: payload.name,
        founding_date: payload.founding_date,
        department_id: payload.department_id,
    };

    let updated = major_repo.update(major_id, updates).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update major: {}", e),
        )
    })?;

    let response = MajorResponse {
        major_id: updated.major_id,
        name: updated.name,
        founding_date: updated.founding_date,
        department_id: updated.department_id,
        create_at: updated.create_at,
        update_at: updated.update_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Delete major (Admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/majors/{major_id}",
    params(
        ("major_id" = Uuid, Path, description = "Major ID")
    ),
    responses(
        (status = 204, description = "Major deleted"),
        (status = 404, description = "Major not found"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Majors"
)]
pub async fn delete_major(
    AuthClaims(auth_claims): AuthClaims,
    Path(major_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Check permission: Admin only (deletion is critical)
    if auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin can delete majors".to_string(),
        ));
    }

    let major_repo = MajorRepository::new();

    major_repo.delete(major_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete major: {}", e),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}
