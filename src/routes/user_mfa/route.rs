use axum::{
    Json, Router,
    http::StatusCode,
    routing::post,
};
use anyhow::Context;
use chrono::{Duration, Utc};
use google_authenticator::GoogleAuthenticator;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use uuid::Uuid;
use crate::routes::user_mfa::dto::{EnableMfaRequestDto, EnableMfaResponseDto, ReqEnableMfaResponseDto};
use crate::extractor::AuthClaims;
use crate::static_service::DATABASE_CONNECTION;
use crate::entities::{user, user_mfa, otp_verify};
use crate::rabbitmq_service::rabbitmq_service::RabbitMQService;
use urlencoding::encode;
use crate::config::{APP_CONFIG, OTP_ISSUER};
use crate::utils::encryption::encrypt;
use crate::utils::gen_otp_code::gen_code;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/user-mfa/enable", post(req_enable_mfa))
        .route("/api/v1/user-mfa/enable-mfa", post(enable_mfa))
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

    let (otp_code, _expires_at) = gen_code().map_err(
        |e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate OTP code: {}", e),
        )
    )?;

    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::minutes(5);

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

    let email_body = format!(
        "Your OTP code to enable MFA is: {:?}. This code will expire in 10 minutes.",
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

#[utoipa::path(
    post,
    tag = "security-settings",
    path = "/api/v1/user-mfa/enable-mfa",
    responses(
        (status = 200, description = "MFA enabled successfully", body = EnableMfaResponseDto),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
    security(
    ("bearer_auth" = [])
    )
)]
#[axum::debug_handler]
pub async fn enable_mfa(
    AuthClaims(claims): AuthClaims,
    Json(body): Json<EnableMfaRequestDto>,
) -> Result<(StatusCode, Json<crate::routes::user_mfa::dto::EnableMfaResponseDto>), (StatusCode, String)> {
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

    let otp_repo = otp_verify::Entity::find()
        .filter(otp_verify::Column::UserId.eq(user_id))
        .filter(otp_verify::Column::Purpose.eq("enable_mfa"))
        .order_by_desc(otp_verify::Column::CreatedAt)
        .limit(1)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to query OTP verify: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "OTP verification not found".to_string(),
            )
        })?;

    if otp_repo.otp_code != body.otp_code {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid OTP code".to_string(),
        ));
    }

    if otp_repo.is_verified {
        return Err((
            StatusCode::BAD_REQUEST,
            "OTP code already verified".to_string(),
        ));
    }

    if otp_repo.expires_at < Utc::now().naive_utc() {
        return Err((
            StatusCode::BAD_REQUEST,
            "OTP code has expired".to_string(),
        ));
    }

    let auth = GoogleAuthenticator::new();
    let secret = auth.create_secret(16);
    let otp_uri = format!(
        "otpauth://totp/{}?secret={}&issuer={}",
        encode(&claims.user_name),
        &secret,
        OTP_ISSUER
    );

    let encode_secret = encrypt(&APP_CONFIG.encryption_key, &secret).map_err(
        |e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encrypt secret: {}", e),
        )
    )?;

    user_mfa::ActiveModel {
        mfa_id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        secret: Set(encode_secret),
        is_enabled: Set(true),
        backup_codes: Default::default(),
        created_at: Default::default(),
        updated_at: Default::default(),
    }
        .insert(db)
        .await
        .context("Failed to create user MFA")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create user MFA: {}", e),
            )
        })?;

    let response = EnableMfaResponseDto {
        message: "Enable MFA successfully".to_string(),
        qr_code: otp_uri,
    };

    Ok((StatusCode::OK, Json(response)))
}