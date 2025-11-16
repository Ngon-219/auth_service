use axum::{Json, Router, http::StatusCode, routing::post};
use axum_extra::TypedHeader;
use axum_extra::headers::{Authorization, authorization::Bearer};

use super::dto::{LoginRequest, LoginResponse, LogoutResponse};
use crate::config::JWT_EXPRIED_TIME;
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::extractor::AuthClaims;
use crate::redis_service::redis_service::JwtBlacklist;
use crate::repositories::{UserMfaRepository, UserRepository};
use do_an_lib::jwt::JwtManager;
use do_an_lib::structs::token_claims::UserRole;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
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

    // Find user by email (this already filters deleted_at IS NULL)
    // If user is deleted, find_by_email will return None
    let user_info = user_repo
        .find_by_email(&payload.email)
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
                "Invalid email or password, or account has been deleted".to_string(),
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

    // Check if user has MFA enabled
    let mfa_repo = UserMfaRepository::new();
    let user_id_str = user_info.user_id.to_string();
    let mfa_enabled = mfa_repo
        .find_enabled_by_user_id(user_info.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check MFA status: {}", e),
            )
        })?;

    // If MFA is enabled, verify the authenticator code
    if let Some(_mfa_record) = mfa_enabled {
        let authenticator_code = payload.authenticator_code.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "MFA is enabled. Please provide authenticator_code".to_string(),
            )
        })?;

        use crate::repositories::mfa_verify_result::MfaVerifyResult;

        let verify_result = mfa_repo
            .verify_mfa_code(&user_id_str, &authenticator_code)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to verify MFA code: {}", e),
                )
            })?;

        match verify_result {
            MfaVerifyResult::Success => {
                // Continue with login
            }
            MfaVerifyResult::Locked { locked_until } => {
                let message = if let Some(until) = locked_until {
                    format!("MFA is locked until {} (too many failed attempts)", until)
                } else {
                    "MFA is locked due to too many failed attempts".to_string()
                };
                return Err((StatusCode::FORBIDDEN, message));
            }
            MfaVerifyResult::CodeAlreadyUsed => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "MFA code has already been used".to_string(),
                ));
            }
            MfaVerifyResult::InvalidCode => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid authenticator code".to_string(),
                ));
            }
            MfaVerifyResult::MfaNotEnabled => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "MFA is not enabled for this user".to_string(),
                ));
            }
        }
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

    let token = jwt_manager
        .create_jwt(
            &user_info.user_id.to_string(),
            &format!("{} {}", user_info.first_name, user_info.last_name),
            user_role,
            JWT_EXPRIED_TIME,
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
        expires_in: JWT_EXPRIED_TIME,
        user_id: user_info.user_id.to_string(),
        email: user_info.email,
        role: role_str.to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Logout endpoint - blacklist JWT token
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = LogoutResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Authentication"
)]
pub async fn logout(
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
    AuthClaims(auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<LogoutResponse>), (StatusCode, String)> {
    let token = bearer.token();
    let user_id = auth_claims.user_id.clone();

    // Add JWT to blacklist
    JwtBlacklist::add_jwt_to_blacklist(&user_id, token)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to blacklist token: {}", e),
            )
        })?;

    let response = LogoutResponse {
        message: "Logout successful".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}
