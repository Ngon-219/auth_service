use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use uuid::Uuid;

use super::dto::{
    CreateDepartmentRequest, DepartmentListResponse, DepartmentResponse, UpdateDepartmentRequest,
};
use crate::extractor::AuthClaims;
use crate::repositories::{DepartmentRepository, DepartmentUpdate};
use do_an_lib::structs::token_claims::UserRole;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/departments", post(create_department))
        .route("/api/v1/departments", get(get_all_departments))
        .route("/api/v1/departments/{department_id}", get(get_department))
        .route(
            "/api/v1/departments/{department_id}",
            put(update_department),
        )
        .route(
            "/api/v1/departments/{department_id}",
            delete(delete_department),
        )
}

/// Create a new department (Admin/Manager only)
#[utoipa::path(
    post,
    path = "/api/v1/departments",
    request_body = CreateDepartmentRequest,
    responses(
        (status = 201, description = "Department created", body = DepartmentResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden - Admin/Manager only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Departments"
)]
pub async fn create_department(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<CreateDepartmentRequest>,
) -> Result<(StatusCode, Json<DepartmentResponse>), (StatusCode, String)> {
    // Check permission: Admin or Manager only
    if auth_claims.role != UserRole::ADMIN && auth_claims.role != UserRole::MANAGER {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin or manager can create departments".to_string(),
        ));
    }

    let dept_repo = DepartmentRepository::new();
    let department_id = Uuid::new_v4();

    let department = dept_repo.create(
        department_id,
        payload.name,
        payload.founding_date,
        payload.dean,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create department: {}", e),
        )
    })?;

    let response = DepartmentResponse {
        department_id: department.department_id,
        name: department.name,
        founding_date: department.founding_date,
        dean: department.dean,
        create_at: department.create_at,
        update_at: department.update_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get all departments (Authenticated users)
#[utoipa::path(
    get,
    path = "/api/v1/departments",
    responses(
        (status = 200, description = "Departments retrieved", body = DepartmentListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Departments"
)]
pub async fn get_all_departments(
    AuthClaims(_auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<DepartmentListResponse>), (StatusCode, String)> {
    let dept_repo = DepartmentRepository::new();

    let departments = dept_repo.find_all().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get departments: {}", e),
        )
    })?;

    let response = DepartmentListResponse {
        total: departments.len(),
        departments: departments
            .into_iter()
            .map(|d| DepartmentResponse {
                department_id: d.department_id,
                name: d.name,
                founding_date: d.founding_date,
                dean: d.dean,
                create_at: d.create_at,
                update_at: d.update_at,
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get department by ID (Authenticated users)
#[utoipa::path(
    get,
    path = "/api/v1/departments/{department_id}",
    params(
        ("department_id" = Uuid, Path, description = "Department ID")
    ),
    responses(
        (status = 200, description = "Department retrieved", body = DepartmentResponse),
        (status = 404, description = "Department not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Departments"
)]
pub async fn get_department(
    AuthClaims(_auth_claims): AuthClaims,
    Path(department_id): Path<Uuid>,
) -> Result<(StatusCode, Json<DepartmentResponse>), (StatusCode, String)> {
    let dept_repo = DepartmentRepository::new();

    let department = dept_repo.find_by_id(department_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get department: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Department not found".to_string()))?;

    let response = DepartmentResponse {
        department_id: department.department_id,
        name: department.name,
        founding_date: department.founding_date,
        dean: department.dean,
        create_at: department.create_at,
        update_at: department.update_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Update department (Admin/Manager only)
#[utoipa::path(
    put,
    path = "/api/v1/departments/{department_id}",
    params(
        ("department_id" = Uuid, Path, description = "Department ID")
    ),
    request_body = UpdateDepartmentRequest,
    responses(
        (status = 200, description = "Department updated", body = DepartmentResponse),
        (status = 404, description = "Department not found"),
        (status = 403, description = "Forbidden - Admin/Manager only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Departments"
)]
pub async fn update_department(
    AuthClaims(auth_claims): AuthClaims,
    Path(department_id): Path<Uuid>,
    Json(payload): Json<UpdateDepartmentRequest>,
) -> Result<(StatusCode, Json<DepartmentResponse>), (StatusCode, String)> {
    // Check permission: Admin or Manager only
    if auth_claims.role != UserRole::ADMIN && auth_claims.role != UserRole::MANAGER {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin or manager can update departments".to_string(),
        ));
    }
    
    let dept_repo = DepartmentRepository::new();

    let updates = DepartmentUpdate {
        name: payload.name,
        founding_date: payload.founding_date,
        dean: payload.dean,
    };

    let updated = dept_repo.update(department_id, updates).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update department: {}", e),
        )
    })?;

    let response = DepartmentResponse {
        department_id: updated.department_id,
        name: updated.name,
        founding_date: updated.founding_date,
        dean: updated.dean,
        create_at: updated.create_at,
        update_at: updated.update_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Delete department (Admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/departments/{department_id}",
    params(
        ("department_id" = Uuid, Path, description = "Department ID")
    ),
    responses(
        (status = 204, description = "Department deleted"),
        (status = 404, description = "Department not found"),
        (status = 403, description = "Forbidden - Admin only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Departments"
)]
pub async fn delete_department(
    AuthClaims(auth_claims): AuthClaims,
    Path(department_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Check permission: Admin only (deletion is critical)
    if auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin can delete departments".to_string(),
        ));
    }
    
    let dept_repo = DepartmentRepository::new();

    dept_repo.delete(department_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete department: {}", e),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}
