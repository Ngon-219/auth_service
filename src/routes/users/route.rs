use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use do_an_lib::structs::token_claims::UserRole;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::fs::File;
use tokio::task;
use uuid::Uuid;
use serde::Serialize;
use utoipa::ToSchema;

use super::dto::{
    BulkUserResponse, CreateUserRequest, CreateUserRequestBulk, UpdateUserRequest, UserCsvColumn,
    UserDetailResponse, UserListResponse, UserQueryParams, UserResponse,
};
use crate::blockchain::{BlockchainService, get_user_private_key};
use crate::config::APP_CONFIG;
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::entities::{major, user_major};
use crate::extractor::AuthClaims;
use crate::middleware::permission;
use crate::rabbitmq_service::consumers::RABBITMQ_CONNECTION;
use crate::rabbitmq_service::rabbitmq_service::RabbitMQService;
use crate::rabbitmq_service::structs::{
    AssignRoleMessage, DeactivateStudentMessage, RegisterNewManagerMessage, RegisterNewUserMessage,
    RemoveManagerMessage,
};
use crate::redis_service::redis_service::{
    helper_get_blockchain_registration_progress, helper_get_current_file_progress,
    BlockchainRegistrationProgress, FileHandleTrackProgress,
};
use crate::repositories::file_upload_repository::FileUploadRepository;
use crate::repositories::{UserRepository, WalletRepository, user_repository::UserUpdate};
use crate::utils::encryption::encrypt_private_key;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BlockchainProgressResponse {
    pub file_upload_history_id: String,
    pub current: u64,
    pub total: u64,
    pub percent: u64,
    pub success: u64,
    pub failed: u64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserProgressResponse {
    pub file_upload_history_id: String,
    pub current: u64,
    pub total: u64,
    pub percent: u64,
    pub success: u64,
    pub failed: u64,
}

async fn fetch_major_names(
    db: &DatabaseConnection,
    major_ids: &[Uuid],
) -> Result<Vec<String>, (StatusCode, String)> {
    if major_ids.is_empty() {
        return Ok(Vec::new());
    }

    let majors = major::Entity::find()
        .filter(major::Column::MajorId.is_in(major_ids.to_vec()))
        .all(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    Ok(majors.into_iter().map(|m| m.name).collect())
}

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/users", post(create_user).get(get_all_users))
        .route("/api/v1/users/bulk", post(create_users_bulk))
        .route(
            "/api/v1/users/bulk/activate-blockchain",
            post(activate_blockchain_registration),
        )
        .route(
            "/api/v1/users/bulk/blockchain-progress/{file_upload_history_id}",
            get(get_blockchain_registration_progress),
        )
        .route(
            "/api/v1/users/bulk/create-progress/{file_upload_history_id}",
            get(get_create_user_progress),
        )
        .route(
            "/api/v1/users/{user_id}",
            get(get_user_by_id).put(update_user).delete(delete_user),
        )
}

/// Handler for creating a single user
#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn create_user(
    AuthClaims(auth_claims): AuthClaims,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let wallet_repo = WalletRepository::new();
    let user_uuid = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;
    let db = user_repo.get_connection();
    let user_private_key = get_user_private_key(db, &user_uuid).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    let hashed_password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to hash password: {}", e),
        )
    })?;

    let (wallet_address, wallet_private_key) =
        BlockchainService::generate_wallet().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate wallet: {}", e),
            )
        })?;

    // Encrypt private key before storing
    let encrypted_private_key =
        encrypt_private_key(&wallet_private_key, &APP_CONFIG.encryption_key).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encrypt private key: {}", e),
            )
        })?;

    let user_id = Uuid::new_v4();
    let wallet_id = Uuid::new_v4();

    let student_code = if payload.role == RoleEnum::Student {
        let lastest_student_code = UserRepository::get_latest_student_code()
            .await
            .expect("Failed to get student code");
        let student_code_i64 = lastest_student_code.parse::<i64>().unwrap_or_default();
        let student_code = student_code_i64 + 1;
        let formated_student_code: String = format!("{:06}", student_code);
        Some(formated_student_code)
    } else {
        None
    };

    let user = user_repo
        .create(
            user_id,
            payload.first_name.clone(),
            payload.last_name.clone(),
            payload.address.clone(),
            payload.email.clone(),
            hashed_password,
            payload.cccd.clone(),
            payload.phone_number.clone(),
            payload.role.clone(),
            false,
            student_code.clone(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create user: {}", e),
            )
        })?;

    wallet_repo
        .create(
            wallet_id,
            user_id,
            wallet_address.clone(),
            encrypted_private_key.clone(),
            APP_CONFIG.chain_type.clone(),
            wallet_address.clone(),
            "active".to_string(),
            APP_CONFIG.chain_id.clone(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create wallet: {}", e),
            )
        })?;

    match payload.role {
        RoleEnum::Student => {
            let full_name = format!("{} {}", payload.first_name, payload.last_name);

            let rabbit_mq_conn = RABBITMQ_CONNECTION
                .get()
                .expect("Failed to get rabbitmq connection");

            let register_user_msg = RegisterNewUserMessage {
                private_key: user_private_key,
                wallet_address: wallet_address.clone(),
                student_code: student_code.unwrap_or_default(),
                full_name,
                email: payload.email,
                file_upload_history_id: None,
            };
            RabbitMQService::publish_to_register_new_user(rabbit_mq_conn, register_user_msg)
                .await
                .map_err(|e| tracing::error!("Failed to publish to register new user: {e}"))
                .ok();
        }
        RoleEnum::Manager => {
            let rabbit_mq_conn = RABBITMQ_CONNECTION
                .get()
                .expect("Failed to get rabbitmq connection");

            let register_new_manager = RegisterNewManagerMessage {
                private_key: user_private_key,
                wallet_address: wallet_address.clone(),
                email: payload.email,
            };

            RabbitMQService::publish_to_register_new_manager(rabbit_mq_conn, register_new_manager)
                .await
                .map_err(|e| tracing::error!("Failed to publish to register new manager: {e}"))
                .ok();
        }
        RoleEnum::Teacher | RoleEnum::Admin => {
            // For Teacher and Admin, use assignRole (requires owner)
            let role_code = match payload.role {
                RoleEnum::Admin => 3,
                RoleEnum::Teacher => 2,
                _ => 0,
            };

            let rabbit_mq_conn = RABBITMQ_CONNECTION.get().ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "RabbitMQ connection not initialized".to_string(),
                )
            })?;

            let assign_role_msg = AssignRoleMessage {
                private_key: user_private_key,
                user_address: wallet_address.clone(),
                role: role_code,
                email: payload.email.clone(),
            };

            RabbitMQService::publish_to_assign_role(rabbit_mq_conn, assign_role_msg)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to publish assign role message: {}", e),
                    )
                })?;
        }
    }

    // Create user-major relationships
    if let Some(major_ids) = payload.major_ids {
        let db = user_repo.get_connection();
        let now = Utc::now().naive_utc();
        for major_id in major_ids.iter() {
            let relationship_model = user_major::ActiveModel {
                user_id: Set(user_id),
                major_id: Set(*major_id),
                create_at: Set(now),
                updated_at: Set(now),
            };

            relationship_model.insert(db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create user-major relationship: {}", e),
                )
            })?;
        }
    }

    let response = UserResponse {
        user_id: user.user_id,
        first_name: user.first_name,
        last_name: user.last_name,
        email: user.email,
        role: user.role,
        wallet_address: wallet_address.clone(),
        wallet_private_key: encrypted_private_key,
        is_first_login: user.is_first_login,
        created_at: user.create_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/bulk",
    request_body = CreateUserRequestBulk,
    responses(
        (status = 201, description = "Bulk user creation completed", body = BulkUserResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn create_users_bulk(
    AuthClaims(_claims): AuthClaims,
    Json(payload): Json<CreateUserRequestBulk>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let file_history_repo = FileUploadRepository::new();
    let file_upload = file_history_repo
        .find_by_id(&payload.history_file_upload_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get file infomation {e}"),
            )
        })?;

    let file_name = file_upload.file_name.clone();
    let file_name_for_task = file_name.clone();

    let result = task::spawn_blocking(move || {
        let file_path = format!("./uploads/{}", file_name_for_task);
        let file = File::open(file_path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to open file: {}", e),
            )
        })?;

        let mut rdr = csv::Reader::from_reader(file);
        let mut users = Vec::new();

        for result in rdr.deserialize() {
            let record: UserCsvColumn = result.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to parse CSV: {}", e),
                )
            })?;
            users.push(record);
        }
        Ok::<Vec<UserCsvColumn>, (StatusCode, String)>(users)
    })
    .await;

    match result {
        Ok(inner_result) => match inner_result {
            Ok(users) => {
                let rabbitmq_conn = RABBITMQ_CONNECTION.get().ok_or_else(|| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "RabbitMQ connection not initialized".to_string(),
                    )
                })?;

                let total_records = users.len() as u64;

                if let Err(err) =
                    FileHandleTrackProgress::set_total_file_handle(&file_name, total_records).await
                {
                    tracing::error!("Failed to set total file handle for {}: {}", file_name, err);
                }

                if let Err(err) =
                    FileHandleTrackProgress::set_current_file_progress(&file_name, 0).await
                {
                    tracing::error!("Failed to reset file progress for {}: {}", file_name, err);
                }

                if let Err(err) =
                    FileHandleTrackProgress::reset_success_failed(&file_name).await
                {
                    tracing::error!(
                        "Failed to reset success/failed counters for {}: {}",
                        file_name,
                        err
                    );
                }

                for (index, mut user) in users.into_iter().enumerate() {
                    user.file_name = Some(file_name.clone());
                    user.row_number = Some((index + 1) as u64);

                    RabbitMQService::publish_to_create_user_db(rabbitmq_conn, user)
                        .await
                        .map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to publish create user message: {}", e),
                            )
                        })?;
                }

                Ok((
                    StatusCode::OK,
                    "Publish batch user to msg queue success".to_string(),
                ))
            }
            Err((status, msg)) => Err((status, msg)),
        },

        Err(join_error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("System error: Task panicked or cancelled - {}", join_error),
        )),
    }
}

/// Get all users with pagination and filtering  
/// Admin can see all, Manager can see students
#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(UserQueryParams),
    responses(
        (status = 200, description = "Users retrieved successfully", body = UserListResponse),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn get_all_users(
    AuthClaims(auth_claims): AuthClaims,
    Query(params): Query<UserQueryParams>,
) -> Result<(StatusCode, Json<UserListResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let wallet_repo = WalletRepository::new();

    // Check permission: Admin or Manager
    permission::is_admin_or_manager(&auth_claims)?;

    let manager_only_students = auth_claims.role == UserRole::MANAGER;
    let (users, total) = user_repo
        .find_all_with_pagination(
            params.page as u32,
            params.page_size as u32,
            params.role.clone(),
            params.search.clone(),
            manager_only_students,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    // Convert to response DTOs
    let mut user_responses = Vec::new();
    for user_model in users {
        // Get wallet info
        let wallet_info = wallet_repo
            .find_by_user_id(user_model.user_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                )
            })?;

        // Get major IDs
        let db = user_repo.get_connection();
        let major_relationships = user_major::Entity::find()
            .filter(user_major::Column::UserId.eq(user_model.user_id))
            .all(db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                )
            })?;

        let major_ids = major_relationships
            .into_iter()
            .map(|m| m.major_id)
            .collect::<Vec<_>>();

        let major_names = fetch_major_names(db, &major_ids).await?;

        user_responses.push(UserDetailResponse {
            user_id: user_model.user_id,
            first_name: user_model.first_name,
            last_name: user_model.last_name,
            address: user_model.address,
            email: user_model.email,
            cccd: user_model.cccd,
            phone_number: user_model.phone_number,
            role: user_model.role,
            is_priority: user_model.is_priority,
            is_first_login: user_model.is_first_login,
            wallet_address: wallet_info.map(|w| w.address),
            major_ids,
            major_names,
            created_at: user_model.create_at,
            updated_at: user_model.update_at,
            student_code: user_model.student_code.unwrap_or("Not Student".to_string())
        });
    }

    Ok((
        StatusCode::OK,
        Json(UserListResponse {
            users: user_responses,
            total: total as usize,
            page: params.page,
            page_size: params.page_size,
        }),
    ))
}

/// Get user by ID
/// Admin can see all, Manager can see students, users can see themselves
#[utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User retrieved successfully", body = UserDetailResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn get_user_by_id(
    AuthClaims(auth_claims): AuthClaims,
    Path(user_id): Path<Uuid>,
) -> Result<(StatusCode, Json<UserDetailResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let db = user_repo.get_connection();

    // Check permission first
    let user_id_str = user_id.to_string();
    if auth_claims.role != UserRole::ADMIN {
        // Non-admin/manager can only see themselves
        if auth_claims.role != UserRole::MANAGER && auth_claims.user_id != user_id_str {
            return Err((
                StatusCode::FORBIDDEN,
                "You can only view your own profile".to_string(),
            ));
        }
    }

    // Get user with wallet and majors
    let (target_user, wallet_info, major_ids) = user_repo
        .get_user_with_wallet_and_majors(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Manager can only see students
    if auth_claims.role == UserRole::MANAGER && target_user.role != RoleEnum::Student {
        return Err((
            StatusCode::FORBIDDEN,
            "Managers can only view student accounts".to_string(),
        ));
    }

    let major_names = fetch_major_names(db, &major_ids).await?;

    let response = UserDetailResponse {
        user_id: target_user.user_id,
        first_name: target_user.first_name,
        last_name: target_user.last_name,
        address: target_user.address,
        email: target_user.email,
        cccd: target_user.cccd,
        major_names,
        phone_number: target_user.phone_number,
        role: target_user.role,
        is_priority: target_user.is_priority,
        is_first_login: target_user.is_first_login,
        wallet_address: wallet_info.map(|w| w.address),
        major_ids,
        created_at: target_user.create_at,
        updated_at: target_user.update_at,
        student_code: target_user.student_code.unwrap_or("Not Student".to_string()),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Update user information (UC10, UC17)
#[utoipa::path(
    put,
    path = "/api/v1/users/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserDetailResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn update_user(
    AuthClaims(auth_claims): AuthClaims,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<(StatusCode, Json<UserDetailResponse>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let db = user_repo.get_connection();

    // Get target user
    let target_user = user_repo
        .find_by_id(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Convert RoleEnum to UserRole for permission check
    let target_role = match target_user.role {
        RoleEnum::Admin => UserRole::ADMIN,
        RoleEnum::Manager => UserRole::MANAGER,
        RoleEnum::Student => UserRole::STUDENT,
        RoleEnum::Teacher => UserRole::TEACHER,
    };

    // Check permission
    permission::can_modify_user(&auth_claims, &target_role)?;

    let hashed_password = if let Some(password) = &payload.password {
        Some(bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to hash password: {}", e),
            )
        })?)
    } else {
        None
    };

    if let Some(_role) = &payload.role {
        // Only admin can change roles
        if auth_claims.role != UserRole::ADMIN {
            return Err((
                StatusCode::FORBIDDEN,
                "Only admin can change user roles".to_string(),
            ));
        }
    }

    let updates = UserUpdate {
        first_name: payload.first_name.clone(),
        last_name: payload.last_name.clone(),
        address: payload.address.clone(),
        email: payload.email.clone(),
        password: hashed_password,
        cccd: payload.cccd.clone(),
        phone_number: payload.phone_number.clone(),
        role: payload.role.clone(),
        is_priority: None,
        is_first_login: None,
    };

    let updated_user = user_repo.update(user_id, updates).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update user: {}", e),
        )
    })?;

    // Update major relationships if provided
    if let Some(major_ids) = payload.major_ids {
        // Delete existing relationships
        user_major::Entity::delete_many()
            .filter(user_major::Column::UserId.eq(user_id))
            .exec(db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update majors: {}", e),
                )
            })?;

        // Create new relationships
        let now = Utc::now().naive_utc();
        for major_id in major_ids.iter() {
            let relationship = user_major::ActiveModel {
                user_id: Set(user_id),
                major_id: Set(*major_id),
                create_at: Set(now),
                updated_at: Set(now),
            };
            relationship.insert(db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create major relationship: {}", e),
                )
            })?;
        }
    }

    // Get updated user with full details
    let (_, wallet_info, major_ids) = user_repo
        .get_user_with_wallet_and_majors(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let major_names = fetch_major_names(db, &major_ids).await?;

    let response = UserDetailResponse {
        user_id: updated_user.user_id,
        first_name: updated_user.first_name,
        last_name: updated_user.last_name,
        address: updated_user.address,
        email: updated_user.email,
        cccd: updated_user.cccd,
        major_names,
        phone_number: updated_user.phone_number,
        role: updated_user.role,
        is_priority: updated_user.is_priority,
        is_first_login: updated_user.is_first_login,
        wallet_address: wallet_info.map(|w| w.address),
        major_ids,
        created_at: updated_user.create_at,
        updated_at: updated_user.update_at,
        student_code: target_user.student_code.unwrap_or("Not Student".to_string()),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Delete user (UC11, UC18)
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User deleted successfully"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn delete_user(
    AuthClaims(auth_claims): AuthClaims,
    Path(user_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, String)> {
    let user_repo = UserRepository::new();
    let wallet_repo = WalletRepository::new();

    // Get target user
    let target_user = user_repo
        .find_by_id(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Convert RoleEnum to UserRole for permission check
    let target_role = match target_user.role {
        RoleEnum::Admin => UserRole::ADMIN,
        RoleEnum::Manager => UserRole::MANAGER,
        RoleEnum::Student => UserRole::STUDENT,
        RoleEnum::Teacher => UserRole::TEACHER,
    };

    // Check permission
    permission::can_modify_user(&auth_claims, &target_role)?;

    // Get wallet address for blockchain operations
    let wallet_info = wallet_repo.find_by_user_id(user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get wallet: {}", e),
        )
    })?;

    let wallet_address = wallet_info.map(|w| w.address).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            "Wallet not found for user".to_string(),
        )
    })?;

    // Get admin/current user private key for blockchain operations
    let db = user_repo.get_connection();
    let admin_user_id = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    let private_key = get_user_private_key(db, &admin_user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get private key: {}", e),
            )
        })?;

    // Publish blockchain message based on role
    let rabbit_mq_conn = RABBITMQ_CONNECTION.get().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RabbitMQ connection not initialized".to_string(),
        )
    })?;

    match target_user.role {
        RoleEnum::Manager => {
            // Remove manager from blockchain
            let message = RemoveManagerMessage {
                private_key,
                manager_address: wallet_address,
                email: target_user.email.clone(),
            };

            RabbitMQService::publish_to_remove_manager(rabbit_mq_conn, message)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to publish remove manager message: {}", e),
                    )
                })?;
        }
        RoleEnum::Student => {
            // Get student_id from blockchain by wallet address
            let blockchain = BlockchainService::new(&private_key).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to initialize blockchain service: {}", e),
                )
            })?;

            let student_id = blockchain
                .get_student_id_by_address(&wallet_address)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to get student ID: {}", e),
                    )
                })?;

            if student_id > 0 {
                // Deactivate student on blockchain
                let message = DeactivateStudentMessage {
                    private_key,
                    student_id,
                    email: target_user.email.clone(),
                };

                RabbitMQService::publish_to_deactivate_student(rabbit_mq_conn, message)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to publish deactivate student message: {}", e),
                        )
                    })?;
            }
        }
        _ => {
            // Admin and Teacher don't need blockchain operations for deletion
        }
    }

    // Soft delete user (set deleted_at instead of hard delete)
    user_repo.soft_delete(user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to soft delete user: {}", e),
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "User deleted successfully",
            "user_id": user_id
        })),
    ))
}

/// Activate blockchain registration for students from CSV file upload
/// This route triggers blockchain registration for all students created from a specific CSV file
#[utoipa::path(
    post,
    path = "/api/v1/users/bulk/activate-blockchain",
    request_body = CreateUserRequestBulk,
    responses(
        (status = 200, description = "Blockchain registration activated successfully"),
        (status = 400, description = "Bad request - no students found or invalid file"),
        (status = 403, description = "Forbidden - Admin/Manager only"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn activate_blockchain_registration(
    AuthClaims(claims): AuthClaims,
    Json(payload): Json<CreateUserRequestBulk>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Permission check: Admin or Manager only
    if claims.role != UserRole::ADMIN && claims.role != UserRole::MANAGER {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admin or manager can activate blockchain registration".to_string(),
        ));
    }

    let file_history_repo = FileUploadRepository::new();
    let file_upload = file_history_repo
        .find_by_id(&payload.history_file_upload_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get file information: {}", e),
            )
        })?;

    let user_repo = UserRepository::new();
    let user_id = Uuid::parse_str(&claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;

    // Read CSV file to get list of emails
    let file_name = file_upload.file_name.clone();
    let file_name_for_task = file_name.clone();

    let csv_emails = task::spawn_blocking(move || {
        let file_path = format!("./uploads/{}", file_name_for_task);
        let file = File::open(file_path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to open file: {}", e),
            )
        })?;

        let mut rdr = csv::Reader::from_reader(file);
        let mut emails = Vec::new();

        for result in rdr.deserialize() {
            let record: UserCsvColumn = result.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to parse CSV: {}", e),
                )
            })?;
            // Only include students
            if record.role.to_lowercase() == "student" {
                emails.push(record.email);
            }
        }
        Ok::<Vec<String>, (StatusCode, String)>(emails)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read CSV file: {}", e),
        )
    })?;

    let emails = csv_emails.map_err(|(status, msg)| (status, msg))?;

    if emails.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No students found in CSV file".to_string(),
        ));
    }

    // Query students by emails from the CSV file
    let students_with_wallets = user_repo
        .find_students_by_emails(emails)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to find students: {}", e),
            )
        })?;

    if students_with_wallets.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No students found for this file upload. Make sure students were created successfully first.".to_string(),
        ));
    }

    // Filter only students (should already be filtered, but double-check)
    let students: Vec<_> = students_with_wallets
        .into_iter()
        .filter(|(user, _)| user.role == RoleEnum::Student)
        .collect();

    if students.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No students found for blockchain registration".to_string(),
        ));
    }

    // Get private key of the user (admin/manager) who will sign the transaction
    let db = user_repo.get_connection();
    let private_key = get_user_private_key(db, &user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get private key: {}", e),
        )
    })?;

    // Set total for progress tracking
    let total_students = students.len() as u64;
    if let Err(err) = BlockchainRegistrationProgress::set_total(
        &payload.history_file_upload_id,
        total_students,
    )
    .await
    {
        tracing::error!(
            "Failed to set total blockchain registration for {}: {}",
            payload.history_file_upload_id,
            err
        );
    }

    if let Err(err) = BlockchainRegistrationProgress::reset_progress(
        &payload.history_file_upload_id,
    )
    .await
    {
        tracing::error!(
            "Failed to reset blockchain registration progress for {}: {}",
            payload.history_file_upload_id,
            err
        );
    }

    // Publish each student individually to blockchain registration queue
    let rabbitmq_conn = RABBITMQ_CONNECTION.get().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RabbitMQ connection not initialized".to_string(),
        )
    })?;

    let mut success_count = 0;
    let mut failed_count = 0;

    for (user, wallet) in students.iter() {
        let student_code = user
            .student_code
            .clone()
            .unwrap_or_else(|| "".to_string());
        let full_name = format!("{} {}", user.first_name, user.last_name);

        let message = RegisterNewUserMessage {
            private_key: private_key.clone(),
            wallet_address: wallet.address.clone(),
            student_code: student_code.clone(),
            full_name: full_name.clone(),
            email: user.email.clone(),
            file_upload_history_id: Some(payload.history_file_upload_id.clone()),
        };

        match RabbitMQService::publish_to_register_new_user(rabbitmq_conn, message).await {
            Ok(_) => {
                success_count += 1;
                tracing::info!(
                    "Published blockchain registration message for student: {} ({})",
                    student_code,
                    user.email
                );
            }
            Err(e) => {
                failed_count += 1;
                tracing::error!(
                    "Failed to publish blockchain registration message for student {} ({}): {}",
                    student_code,
                    user.email,
                    e
                );
            }
        }
    }

    if failed_count > 0 {
        return Err((
            StatusCode::PARTIAL_CONTENT,
            format!(
                "Blockchain registration activated for {} students, {} failed",
                success_count, failed_count
            ),
        ));
    }

    Ok((
        StatusCode::OK,
        format!(
            "Blockchain registration activated for {} students",
            success_count
        ),
    ))
}

/// Get blockchain registration progress for a file upload
#[utoipa::path(
    get,
    path = "/api/v1/users/bulk/blockchain-progress/{file_upload_history_id}",
    params(
        ("file_upload_history_id" = String, Path, description = "File upload history ID")
    ),
    responses(
        (status = 200, description = "Progress retrieved successfully", body = BlockchainProgressResponse),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn get_blockchain_registration_progress(
    AuthClaims(_claims): AuthClaims,
    Path(file_upload_history_id): Path<String>,
) -> Result<(StatusCode, Json<BlockchainProgressResponse>), (StatusCode, String)> {
    let progress = helper_get_blockchain_registration_progress(&file_upload_history_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch blockchain registration progress: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(BlockchainProgressResponse {
            file_upload_history_id,
            current: progress.current,
            total: progress.total,
            percent: progress.percent,
            success: progress.success,
            failed: progress.failed,
        }),
    ))
}

/// Get create-user (DB) progress for a file upload
#[utoipa::path(
    get,
    path = "/api/v1/users/bulk/create-progress/{file_upload_history_id}",
    params(
        ("file_upload_history_id" = String, Path, description = "File upload history ID")
    ),
    responses(
        (status = 200, description = "Progress retrieved successfully", body = CreateUserProgressResponse),
        (status = 404, description = "File upload history not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub async fn get_create_user_progress(
    AuthClaims(_claims): AuthClaims,
    Path(file_upload_history_id): Path<String>,
) -> Result<(StatusCode, Json<CreateUserProgressResponse>), (StatusCode, String)> {
    let file_repo = FileUploadRepository::new();
    let file_record = file_repo
        .find_by_id(&file_upload_history_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                format!("Failed to find file upload record: {}", e),
            )
        })?;

    let progress = helper_get_current_file_progress(&file_record.file_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch create user progress: {}", e),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(CreateUserProgressResponse {
            file_upload_history_id,
            current: progress.current,
            total: progress.total,
            percent: progress.percent,
            success: progress.success,
            failed: progress.failed,
        }),
    ))
}
