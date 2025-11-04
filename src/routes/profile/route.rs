use super::dto::ProfileResponse;
use crate::blockchain::get_user_blockchain_service;
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::extractor::AuthClaims;
use crate::repositories::{UserRepository, WalletRepository};
use axum::{Json, Router, http::StatusCode, routing::get};
use uuid::Uuid;

pub fn create_route() -> Router {
    Router::new().route("/api/v1/profile", get(get_profile))
}

/// Get current user profile (requires JWT)
/// Also demonstrates using user's wallet to call blockchain
#[utoipa::path(
    get,
    path = "/api/v1/profile",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Profile retrieved", body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Profile"
)]
pub async fn get_profile(
    AuthClaims(auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<ProfileResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let wallet_repo = WalletRepository::new();

    let user_id_uuid = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    // Get user info from DB
    let user_info = user_repo.find_by_id(user_id_uuid)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Get wallet info
    let wallet_info = wallet_repo.find_by_user_id(user_id_uuid)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Wallet not found".to_string()))?;

    // Create blockchain service with user's private key
    let db = user_repo.get_connection();
    let user_blockchain = get_user_blockchain_service(db, &user_id_uuid)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create blockchain service: {}", e),
            )
        })?;

    // Call blockchain contract using user's wallet
    let blockchain_role = user_blockchain
        .get_user_role(&wallet_info.address)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get blockchain role: {}", e),
            )
        })?;

    let is_active = if user_info.role == RoleEnum::Student {
        user_blockchain
            .is_active_student(&wallet_info.address)
            .await
            .unwrap_or(false)
    } else {
        true
    };

    let role_str = match user_info.role {
        RoleEnum::Admin => "admin",
        RoleEnum::Manager => "manager",
        RoleEnum::Student => "student",
        RoleEnum::Teacher => "teacher",
    };

    let response = ProfileResponse {
        user_id: user_info.user_id,
        first_name: user_info.first_name,
        last_name: user_info.last_name,
        email: user_info.email,
        role: role_str.to_string(),
        wallet_address: wallet_info.address,
        blockchain_role,
        is_active,
    };

    Ok((StatusCode::OK, Json(response)))
}
