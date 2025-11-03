use axum::{
    Json, Router,
    http::StatusCode,
    routing::post,
};
use anyhow::Context;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
use crate::routes::user_mfa::dto::ReqEnableMfaResponseDto;
use crate::extractor::AuthClaims;
use crate::static_service::DATABASE_CONNECTION;
use crate::entities::{user, user_mfa, otp_verify};
use crate::rabbitmq_service::rabbitmq_service::RabbitMQService;
use rand::Rng;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/user-mfa/enable", post(req_enable_mfa))
}

#[utoipa::path(
    post,
    tag = "security-settings",
    path = "/api/v1/user-mfa/enable",
    responses(
        (status = 201, description = "OTP code sent to email", body = ReqEnableMfaResponseDto),
        (status = 400, description = "User already has MFA enabled"),
        (status = 500, description = "Internal server error"),
    ),
    security(
    ("bearer_auth" = [])
    )
)]
#[axum::debug_handler]
pub async fn req_enable_mfa(
    AuthClaims(claims): AuthClaims,
) -> Result<(StatusCode, Json<ReqEnableMfaResponseDto>), (StatusCode, String)> {
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    let user_id = Uuid::parse_str(&claims.user_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid user_id: {}", e),
            )
        })?;

    // Find user
    let user_info = user::Entity::find()
        .filter(user::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query database: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "User not found".to_string(),
            )
        })?;

    // Check if MFA is already enabled
    let existing_mfa = user_mfa::Entity::find()
        .filter(user_mfa::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check user mfa: {}", e),
            )
        })?;

    if let Some(mfa_record) = existing_mfa {
        if mfa_record.is_enabled {
            return Err((
                StatusCode::BAD_REQUEST,
                "MFA is already enabled".to_string(),
            ));
        }
    }

    // Generate 6-digit OTP code (before any await points to avoid Send issues)
    let otp_code = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(100000..=999999))
    };

    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::minutes(10); // OTP expires in 10 minutes

    // Create OTP verify record
    let otp_verify_active_model = otp_verify::ActiveModel {
        otp_id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        otp_code: Set(otp_code.clone()),
        email: Set(user_info.email.clone()),
        purpose: Set("enable_mfa".to_string()),
        is_verified: Set(false),
        expires_at: Set(expires_at),
        created_at: Set(now),
        updated_at: Set(now),
    };

    otp_verify_active_model
        .insert(db)
        .await
        .context("Failed to create otp verify")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create OTP verify: {}", e),
            )
        })?;

    // Send OTP code via email
    let email_body = format!(
        "Your OTP code to enable MFA is: {}. This code will expire in 10 minutes.",
        otp_code
    );

    let rabbit_mq_connection = RabbitMQService::new().await;
    RabbitMQService::publish_to_mail_queue(
        rabbit_mq_connection,
        &user_info.email,
        "Enable MFA - Verification Code",
        &email_body,
    )
    .await
    .context("Failed to send email")
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to send email: {}", e),
        )
    })?;

    let response = ReqEnableMfaResponseDto {
        message: "Check your email to enable mfa".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}