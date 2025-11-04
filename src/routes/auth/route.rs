use axum::{Json, Router, http::StatusCode, routing::post};

use super::dto::{LoginRequest, LoginResponse};
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::repositories::UserRepository;
use do_an_lib::jwt::JwtManager;
use do_an_lib::structs::token_claims::UserRole;

pub fn create_route() -> Router {
    Router::new().route("/api/v1/auth/login", post(login))
}

/// Login endpoint - returns JWT token
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
)]
pub async fn login(
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();

    // Find user by email
    let user_info = user_repo.find_by_email(&payload.email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid email or password".to_string(),
            )
        })?;

    // Verify password
    let password_valid = bcrypt::verify(&payload.password, &user_info.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Password verification error: {}", e),
        )
    })?;

    if !password_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    }

    // Convert RoleEnum to UserRole
    let user_role = match user_info.role {
        RoleEnum::Admin => UserRole::ADMIN,
        RoleEnum::Manager => UserRole::MANAGER,
        RoleEnum::Student => UserRole::STUDENT,
        RoleEnum::Teacher => UserRole::TEACHER,
    };

    // Get JWT secret from config (you should use APP_CONFIG.jwt_secret)
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_key".to_string());
    let jwt_manager = JwtManager::new(jwt_secret);

    // Create token with 24 hours expiration
    let expires_in = 86400i64; // 24 hours in seconds

    let token = jwt_manager
        .create_jwt(
            &user_info.user_id.to_string(),
            &format!("{} {}", user_info.first_name, user_info.last_name),
            user_role,
            expires_in,
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create token: {}", e),
            )
        })?;

    let role_str = match user_info.role {
        RoleEnum::Admin => "admin",
        RoleEnum::Manager => "manager",
        RoleEnum::Student => "student",
        RoleEnum::Teacher => "teacher",
    };

    let response = LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        user_id: user_info.user_id.to_string(),
        email: user_info.email,
        role: role_str.to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}
