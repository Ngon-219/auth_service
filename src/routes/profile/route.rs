use super::dto::{DepartmentInfo, MajorInfo, ProfileResponse, UserInfo, WalletInfo};
use crate::blockchain::get_user_blockchain_service;
use crate::entities::{
    sea_orm_active_enums::RoleEnum,
    department,
    major,
};
use crate::extractor::AuthClaims;
use crate::repositories::UserRepository;
use axum::{http::StatusCode, routing::get, Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
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

    let user_id_uuid = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    // Get user info, wallet and majors
    let (user_info, wallet_info_opt, major_ids) = user_repo
        .get_user_with_wallet_and_majors(user_id_uuid)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let wallet_info = wallet_info_opt
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Wallet not found".to_string()))?;

    let db = user_repo.get_connection();

    // Load majors for the user
    let majors = if major_ids.is_empty() {
        Vec::new()
    } else {
        major::Entity::find()
            .filter(major::Column::MajorId.is_in(major_ids.clone()))
            .all(db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to load majors: {}", e),
                )
            })?
    };

    // Collect departments referenced by majors
    let mut department_ids: Vec<Uuid> = majors
        .iter()
        .filter_map(|m| m.department_id)
        .collect();
    department_ids.sort();
    department_ids.dedup();

    let departments = if department_ids.is_empty() {
        Vec::new()
    } else {
        department::Entity::find()
            .filter(department::Column::DepartmentId.is_in(department_ids.clone()))
            .all(db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to load departments: {}", e),
                )
            })?
    };

    // Create blockchain service with user's private key
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

    let user_response = UserInfo {
        user_id: user_info.user_id,
        first_name: user_info.first_name.clone(),
        last_name: user_info.last_name.clone(),
        address: user_info.address.clone(),
        email: user_info.email.clone(),
        phone_number: user_info.phone_number.clone(),
        cccd: user_info.cccd.clone(),
        is_priority: user_info.is_priority,
        is_first_login: user_info.is_first_login,
        role: user_info.role.clone(),
        status: user_info.status,
        student_code: user_info.student_code.clone(),
        created_at: user_info.create_at,
        updated_at: user_info.update_at,
    };

    let wallet_response = WalletInfo {
        wallet_id: wallet_info.wallet_id,
        address: wallet_info.address.clone(),
        chain_type: wallet_info.chain_type.clone(),
        public_key: wallet_info.public_key.clone(),
        status: wallet_info.status.clone(),
        network_id: wallet_info.network_id.clone(),
        last_used_at: wallet_info.last_used_at,
        created_at: wallet_info.created_at,
        updated_at: wallet_info.updated_at,
    };

    let majors_response = majors
        .iter()
        .map(|major_model| MajorInfo {
            major_id: major_model.major_id,
            name: major_model.name.clone(),
            department_id: major_model.department_id,
            founding_date: major_model.founding_date,
            created_at: major_model.create_at,
            updated_at: major_model.update_at,
        })
        .collect();

    let departments_response = departments
        .into_iter()
        .map(|department_model| DepartmentInfo {
            department_id: department_model.department_id,
            name: department_model.name,
            dean: department_model.dean,
            founding_date: department_model.founding_date,
            created_at: department_model.create_at,
            updated_at: department_model.update_at,
        })
        .collect();

    let response = ProfileResponse {
        user: user_response,
        wallet: wallet_response,
        majors: majors_response,
        departments: departments_response,
        blockchain_role,
        is_active,
    };

    Ok((StatusCode::OK, Json(response)))
}
