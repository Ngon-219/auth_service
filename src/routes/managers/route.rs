use super::dto::{
    AddManagerRequest, CheckManagerRequest, ManagerListResponse, ManagerResponse,
    RemoveManagerRequest,
};
use crate::blockchain::get_user_blockchain_service;
use crate::extractor::AuthClaims;
use crate::repositories::UserRepository;
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{delete, get, post},
};
use do_an_lib::structs::token_claims::UserRole;
use uuid::Uuid;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/managers", post(add_manager))
        .route("/api/v1/managers", delete(remove_manager))
        .route("/api/v1/managers", get(get_all_managers))
        .route("/api/v1/managers/check", post(check_manager))
}

/// Add a manager to the blockchain (Admin only)
/// Requires onlyOwner permission in smart contract
#[utoipa::path(
    post,
    path = "/api/v1/managers",
    request_body = AddManagerRequest,
    responses(
        (status = 200, description = "Manager added successfully", body = ManagerResponse),
        (status = 403, description = "Forbidden - Admin/Owner only"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Managers"
)]
pub async fn add_manager(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<AddManagerRequest>,
) -> Result<(StatusCode, Json<ManagerResponse>), (StatusCode, String)> {
    // Permission check: Admin only (onlyOwner on smart contract)
    if auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin can add managers".to_string(),
        ));
    }

    let user_repo = UserRepository::new();
    let user_id = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;
    let db = user_repo.get_connection();
    let blockchain = get_user_blockchain_service(db, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to initialize blockchain service: {}", e),
            )
        })?;

    blockchain
        .add_manager(&payload.manager_address)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to add manager: {}", e),
            )
        })?;

    let response = ManagerResponse {
        address: payload.manager_address,
        is_manager: true,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Remove a manager from the blockchain (Admin only)
/// Requires onlyOwner permission in smart contract
#[utoipa::path(
    delete,
    path = "/api/v1/managers",
    request_body = RemoveManagerRequest,
    responses(
        (status = 200, description = "Manager removed successfully", body = ManagerResponse),
        (status = 403, description = "Forbidden - Admin/Owner only"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Managers"
)]
pub async fn remove_manager(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<RemoveManagerRequest>,
) -> Result<(StatusCode, Json<ManagerResponse>), (StatusCode, String)> {
    // Permission check: Admin only (onlyOwner on smart contract)
    if auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin can remove managers".to_string(),
        ));
    }

    let user_repo = UserRepository::new();
    let user_id = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    let db = user_repo.get_connection();
    let blockchain = get_user_blockchain_service(db, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to initialize blockchain service: {}", e),
            )
        })?;

    blockchain
        .remove_manager(&payload.manager_address)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to remove manager: {}", e),
            )
        })?;

    let response = ManagerResponse {
        address: payload.manager_address,
        is_manager: false,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Get all managers from the blockchain (Authenticated users)
#[utoipa::path(
    get,
    path = "/api/v1/managers",
    responses(
        (status = 200, description = "Managers retrieved successfully", body = ManagerListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Managers"
)]
pub async fn get_all_managers(
    AuthClaims(auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<ManagerListResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let user_id = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    let db = user_repo.get_connection();
    let blockchain = get_user_blockchain_service(db, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to initialize blockchain service: {}", e),
            )
        })?;

    let managers = blockchain.get_all_managers().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get managers: {}", e),
        )
    })?;

    let count = blockchain.get_manager_count().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get manager count: {}", e),
        )
    })?;

    let response = ManagerListResponse {
        managers,
        total_count: count,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Check if an address is a manager
#[utoipa::path(
    post,
    path = "/api/v1/managers/check",
    request_body = CheckManagerRequest,
    responses(
        (status = 200, description = "Manager check completed", body = ManagerResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Managers"
)]
pub async fn check_manager(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<CheckManagerRequest>,
) -> Result<(StatusCode, Json<ManagerResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let user_id = uuid::Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;
    let db = user_repo.get_connection();
    let blockchain = get_user_blockchain_service(db, &user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to initialize blockchain service: {}", e),
            )
        })?;

    let is_manager = blockchain.is_manager(&payload.address).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to check manager: {}", e),
        )
    })?;

    let response = ManagerResponse {
        address: payload.address,
        is_manager,
    };

    Ok((StatusCode::OK, Json(response)))
}
