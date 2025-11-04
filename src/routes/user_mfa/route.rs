use axum::{
    Json, Router,
    http::StatusCode,
    routing::post,
};
use anyhow::Context;
use chrono::{Duration, Utc};
use google_authenticator::GoogleAuthenticator;
use uuid::Uuid;
use crate::routes::user_mfa::dto::{EnableMfaRequestDto, EnableMfaResponseDto, ReqEnableMfaResponseDto, VerifyMfaCodeTestRequestDto, VerifyMfaCodeTestResponseDto};
use crate::extractor::AuthClaims;
use crate::repositories::{UserRepository, UserMfaRepository, OtpVerifyRepository};
use crate::rabbitmq_service::rabbitmq_service::RabbitMQService;
use urlencoding::encode;
use crate::config::{APP_CONFIG, OTP_ISSUER};
use crate::utils::encryption::encrypt;
use crate::utils::gen_otp_code::gen_code;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/user-mfa/enable", post(req_enable_mfa))
        .route("/api/v1/user-mfa/enable-mfa", post(enable_mfa))
        .route("/api/v1/user-mfa/verify", post(verify_mfa_code_test))
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
    let user_repo = UserRepository::new();
    let mfa_repo = UserMfaRepository::new();
    let otp_repo = OtpVerifyRepository::new();

    let user_id = Uuid::parse_str(&claims.user_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid user_id: {}", e),
            )
        })?;

    // Find user
    let user_info = user_repo.find_by_id(user_id)
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
    let existing_mfa = mfa_repo.find_by_user_id(user_id)
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

    let expires_at = Utc::now().naive_utc() + Duration::minutes(5);

    otp_repo.create(
        user_id,
        otp_code.clone(),
        user_info.email.clone(),
        "enable_mfa".to_string(),
        expires_at,
    )
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
    let otp_repo = OtpVerifyRepository::new();
    let mfa_repo = UserMfaRepository::new();

    let user_id = Uuid::parse_str(&claims.user_id)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid user_id: {}", e),
            )
        })?;

    let otp_record = otp_repo.find_latest_by_user_and_purpose(user_id, "enable_mfa")
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

    if otp_record.otp_code != body.otp_code {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid OTP code".to_string(),
        ));
    }

    if otp_record.is_verified {
        return Err((
            StatusCode::BAD_REQUEST,
            "OTP code already verified".to_string(),
        ));
    }

    if otp_record.expires_at < Utc::now().naive_utc() {
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

    mfa_repo.create(
        user_id,
        encode_secret,
        None, // backup_codes
    )
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

/// Test endpoint to verify MFA code (for testing only)
#[utoipa::path(
    post,
    tag = "security-settings",
    path = "/api/v1/user-mfa/verify",
    request_body = VerifyMfaCodeTestRequestDto,
    responses(
        (status = 200, description = "MFA code verification result", body = VerifyMfaCodeTestResponseDto),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_auth" = []))
)]
#[axum::debug_handler]
pub async fn verify_mfa_code_test(
    AuthClaims(claims): AuthClaims,
    Json(body): Json<VerifyMfaCodeTestRequestDto>,
) -> Result<(StatusCode, Json<VerifyMfaCodeTestResponseDto>), (StatusCode, String)> {
    let mfa_repo = UserMfaRepository::new();

    use crate::repositories::mfa_verify_result::MfaVerifyResult;
    
    let result = mfa_repo.verify_mfa_code(&claims.user_id, &body.authenticator_code)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to verify MFA code: {}", e),
            )
        })?;

    let (is_valid, message, reason, locked_until) = match result {
        MfaVerifyResult::Success => (
            true,
            "MFA code verified successfully".to_string(),
            "success".to_string(),
            None,
        ),
        MfaVerifyResult::Locked { locked_until } => (
            false,
            format!("MFA is locked{}", 
                locked_until.map(|u| format!(" until {}", u)).unwrap_or_default()
            ),
            "locked".to_string(),
            locked_until,
        ),
        MfaVerifyResult::CodeAlreadyUsed => (
            false,
            "MFA code has already been used".to_string(),
            "code_already_used".to_string(),
            None,
        ),
        MfaVerifyResult::InvalidCode => (
            false,
            "Invalid MFA code".to_string(),
            "invalid_code".to_string(),
            None,
        ),
        MfaVerifyResult::MfaNotEnabled => (
            false,
            "MFA is not enabled for this user".to_string(),
            "mfa_not_enabled".to_string(),
            None,
        ),
    };

    let response = VerifyMfaCodeTestResponseDto {
        is_valid,
        message,
        reason,
        locked_until,
    };

    Ok((StatusCode::OK, Json(response)))
}