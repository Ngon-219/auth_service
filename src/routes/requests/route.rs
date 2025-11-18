use super::dto::{
    CreateRequestRequest, RequestListResponse, RequestQueryParams, RequestResponse,
    ScheduleRequestRequest, ScheduleRequestResponse,
};
use crate::entities::sea_orm_active_enums::RequestStatus;
use crate::extractor::AuthClaims;
use crate::rabbitmq_service::consumers::get_rabbitmq_connetion;
use crate::rabbitmq_service::rabbitmq_service::RabbitMQService;
use crate::repositories::mfa_verify_result::MfaVerifyResult;
use crate::repositories::{RequestRepository, UserMfaRepository, UserRepository};
use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use do_an_lib::structs::token_claims::UserRole;

pub fn create_route() -> Router {
    Router::new()
        .route(
            "/api/v1/requests",
            post(create_request).get(get_my_requests),
        )
        .route(
            "/api/v1/requests/{request_id}/schedule",
            post(schedule_request),
        )
        .route("/api/v1/requests/all", get(get_all_requests))
}

#[utoipa::path(
    post,
    path = "/api/v1/requests",
    request_body = CreateRequestRequest,
    responses(
        (status = 201, description = "Request created successfully", body = RequestResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Requests"
)]
pub async fn create_request(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<CreateRequestRequest>,
) -> Result<(StatusCode, Json<RequestResponse>), (StatusCode, String)> {
    let user_id = uuid::Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    if payload.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Content cannot be empty".to_string(),
        ));
    }

    // Check if user has MFA enabled and verify code
    let mfa_repo = UserMfaRepository::new();
    let mfa_enabled = mfa_repo
        .find_enabled_by_user_id(user_id)
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

        let verify_result = mfa_repo
            .verify_mfa_code(&auth_claims.user_id, &authenticator_code)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to verify MFA code: {}", e),
                )
            })?;

        match verify_result {
            MfaVerifyResult::Success => {
                // Continue with request creation
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

    let request_repo = RequestRepository::new();
    let request = request_repo
        .create(user_id, payload.content)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create request: {}", e),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(RequestResponse {
            request_id: request.request_id.to_string(),
            user_id: request.user_id.to_string(),
            content: request.content,
            status: request.status,
            scheduled_at: request.scheduled_at.map(|d| d.to_string()),
            created_at: request.created_at.to_string(),
            updated_at: request.updated_at.to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/requests",
    responses(
        (status = 200, description = "Requests retrieved successfully", body = RequestListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Requests"
)]
pub async fn get_my_requests(
    AuthClaims(auth_claims): AuthClaims,
) -> Result<(StatusCode, Json<RequestListResponse>), (StatusCode, String)> {
    let user_id = uuid::Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    let request_repo = RequestRepository::new();
    let requests = request_repo.find_by_user_id(user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get requests: {}", e),
        )
    })?;

    let request_responses: Vec<RequestResponse> = requests
        .into_iter()
        .map(|r| RequestResponse {
            request_id: r.request_id.to_string(),
            user_id: r.user_id.to_string(),
            content: r.content,
            status: r.status,
            scheduled_at: r.scheduled_at.map(|d| d.to_string()),
            created_at: r.created_at.to_string(),
            updated_at: r.updated_at.to_string(),
        })
        .collect();

    // For get_my_requests, we don't have pagination info, so set defaults
    let total = request_responses.len() as u64;
    let total_pages = 1;

    Ok((
        StatusCode::OK,
        Json(RequestListResponse {
            requests: request_responses,
            total,
            page: 1,
            page_size: total as u32,
            total_pages,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/requests/{request_id}/schedule",
    request_body = ScheduleRequestRequest,
    responses(
        (status = 200, description = "Request scheduled and email sent successfully", body = ScheduleRequestResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden - Manager only"),
        (status = 404, description = "Request not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Requests"
)]
pub async fn schedule_request(
    AuthClaims(auth_claims): AuthClaims,
    Path(request_id): Path<String>,
    Json(payload): Json<ScheduleRequestRequest>,
) -> Result<(StatusCode, Json<ScheduleRequestResponse>), (StatusCode, String)> {
    // Permission check: Manager or Admin only
    if auth_claims.role != UserRole::MANAGER && auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only managers and admins can schedule requests".to_string(),
        ));
    }

    let manager_user_id = uuid::Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    // Check if manager has MFA enabled and verify code
    let mfa_repo = UserMfaRepository::new();
    let mfa_enabled = mfa_repo
        .find_enabled_by_user_id(manager_user_id)
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

        let verify_result = mfa_repo
            .verify_mfa_code(&auth_claims.user_id, &authenticator_code)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to verify MFA code: {}", e),
                )
            })?;

        match verify_result {
            MfaVerifyResult::Success => {
                // Continue with scheduling
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

    let request_uuid = uuid::Uuid::parse_str(&request_id).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid request_id: {}", e),
        )
    })?;

    // Parse scheduled_at
    let scheduled_at = NaiveDateTime::parse_from_str(&payload.scheduled_at, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&payload.scheduled_at, "%Y-%m-%d %H:%M:%S"))
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid scheduled_at format. Expected YYYY-MM-DDTHH:MM:SS or YYYY-MM-DD HH:MM:SS: {}", e),
            )
        })?;

    let request_repo = RequestRepository::new();

    // Get request to find user
    let request = request_repo
        .find_by_id(request_uuid)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get request: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Request not found".to_string()))?;

    // Update request status and scheduled_at
    let updated_request = request_repo
        .update_status_and_schedule(request_uuid, RequestStatus::Scheduled, Some(scheduled_at))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update request: {}", e),
            )
        })?;

    // Get user info to send email
    let user_repo = UserRepository::new();
    let user = user_repo
        .find_by_id(request.user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get user: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Send email via RabbitMQ
    let rabbitmq_conn = get_rabbitmq_connetion().await;
    let email_subject = "Lịch hẹn xử lý yêu cầu";
    let email_body = format!(
        "Xin chào {},\n\nYêu cầu của bạn đã được lên lịch xử lý vào thời gian: {}\n\nNội dung yêu cầu: {}\n\n{}\n\nTrân trọng,\nHệ thống quản lý",
        user.first_name,
        scheduled_at.format("%d/%m/%Y %H:%M"),
        request.content,
        payload.message.as_deref().unwrap_or("")
    );

    RabbitMQService::publish_to_mail_queue(rabbitmq_conn, &user.email, email_subject, &email_body)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to send email: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(ScheduleRequestResponse {
            success: true,
            message: "Request scheduled and email sent successfully".to_string(),
            request: RequestResponse {
                request_id: updated_request.request_id.to_string(),
                user_id: updated_request.user_id.to_string(),
                content: updated_request.content,
                status: updated_request.status,
                scheduled_at: updated_request.scheduled_at.map(|d| d.to_string()),
                created_at: updated_request.created_at.to_string(),
                updated_at: updated_request.updated_at.to_string(),
            },
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/requests/all",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("page_size" = Option<u32>, Query, description = "Page size (default: 20)"),
        ("status" = Option<RequestStatus>, Query, description = "Filter by status")
    ),
    responses(
        (status = 200, description = "All requests retrieved successfully", body = RequestListResponse),
        (status = 403, description = "Forbidden - Manager/Admin only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Requests"
)]
pub async fn get_all_requests(
    AuthClaims(auth_claims): AuthClaims,
    Query(params): Query<RequestQueryParams>,
) -> Result<(StatusCode, Json<RequestListResponse>), (StatusCode, String)> {
    // Permission check: Manager or Admin only
    if auth_claims.role != UserRole::MANAGER && auth_claims.role != UserRole::ADMIN {
        return Err((
            StatusCode::FORBIDDEN,
            "Only managers and admins can view all requests".to_string(),
        ));
    }

    // Validate pagination parameters
    let page = if params.page == 0 { 1 } else { params.page };
    let page_size = if params.page_size == 0 || params.page_size > 100 {
        20
    } else {
        params.page_size
    };

    let request_repo = RequestRepository::new();
    let (requests, total) = request_repo
        .find_all_with_pagination(page, page_size, params.status)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get requests: {}", e),
            )
        })?;

    let request_responses: Vec<RequestResponse> = requests
        .into_iter()
        .map(|r| RequestResponse {
            request_id: r.request_id.to_string(),
            user_id: r.user_id.to_string(),
            content: r.content,
            status: r.status,
            scheduled_at: r.scheduled_at.map(|d| d.to_string()),
            created_at: r.created_at.to_string(),
            updated_at: r.updated_at.to_string(),
        })
        .collect();

    let total_pages = (total as f64 / page_size as f64).ceil() as u64;

    Ok((
        StatusCode::OK,
        Json(RequestListResponse {
            requests: request_responses,
            total,
            page,
            page_size,
            total_pages,
        }),
    ))
}
