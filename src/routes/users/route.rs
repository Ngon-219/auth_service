use axum::{
    Json, Router,
    extract::{Multipart, Path, Query},
    http::StatusCode,
    routing::{get, post},
};
use calamine::{DataType, Reader, Xlsx, open_workbook_from_rs};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use std::io::Cursor;
use uuid::Uuid;
use do_an_lib::structs::token_claims::UserRole;

use super::dto::{
    BulkUserError, BulkUserResponse, CreateUserRequest, ExcelUserRow,
    UpdateUserRequest, UserDetailResponse, UserListResponse, UserQueryParams, UserResponse
};
use crate::blockchain::{BlockchainService, get_admin_blockchain_service, get_user_private_key};
use crate::entities::sea_orm_active_enums::RoleEnum;
use crate::entities::{user, user_major, wallet};
use crate::extractor::AuthClaims;
use crate::middleware::permission;
use crate::static_service::DATABASE_CONNECTION;
use crate::utils::encryption::encrypt_private_key;
use crate::config::APP_CONFIG;

pub fn create_route() -> Router {
    Router::new()
        .route("/api/v1/users", post(create_user).get(get_all_users))
        .route("/api/v1/users/bulk", post(create_users_bulk))
        .route(
            "/api/v1/users/{user_id}",
            get(get_user_by_id)
                .put(update_user)
                .delete(delete_user)
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
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");
    let user_uuid = Uuid::parse_str(&auth_claims.user_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;
    let user_private_key = get_user_private_key(db, &user_uuid).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Invalid user_id: {}", e),
        )
    })?;
    let blockchain = BlockchainService::new(&user_private_key)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", e),
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
    let encrypted_private_key = encrypt_private_key(&wallet_private_key, &APP_CONFIG.encryption_key)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encrypt private key: {}", e),
            )
        })?;

    let user_id = Uuid::new_v4();
    let wallet_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    let user_model = user::ActiveModel {
        user_id: Set(user_id),
        first_name: Set(payload.first_name.clone()),
        last_name: Set(payload.last_name.clone()),
        address: Set(payload.address.clone()),
        email: Set(payload.email.clone()),
        password: Set(hashed_password),
        is_priority: Set(false),
        cccd: Set(payload.cccd.clone()),
        phone_number: Set(payload.phone_number.clone()),
        is_first_login: Set(true),
        create_at: Set(now),
        update_at: Set(now),
        role: Set(payload.role.clone()),
    };

    let user = user_model.insert(db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create user: {}", e),
        )
    })?;

    let wallet_model = wallet::ActiveModel {
        wallet_id: Set(wallet_id),
        user_id: Set(user_id),
        address: Set(wallet_address.clone()),
        private_key: Set(encrypted_private_key),
        chain_type: Set("ethereum".to_string()),
        public_key: Set(wallet_address.clone()),
        status: Set("active".to_string()),
        network_id: Set("1".to_string()), // Mainnet by default
        last_used_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    wallet_model.insert(db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create wallet: {}", e),
        )
    })?;

    // Register on blockchain
    match payload.role {
        RoleEnum::Student => {
            let student_code = payload.student_code.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "Student code is required for students".to_string(),
                )
            })?;

            let full_name = format!("{} {}", payload.first_name, payload.last_name);

            blockchain
                .register_student(&wallet_address, &student_code, &full_name, &payload.email)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to register student on blockchain: {}", e),
                    )
                })?;
        }
        RoleEnum::Manager => {
            // Use addManager instead of assignRole for managers
            blockchain.add_manager(&wallet_address).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to add manager on blockchain: {}", e),
                )
            })?;
        }
        RoleEnum::Teacher | RoleEnum::Admin => {
            // For Teacher and Admin, use assignRole (requires owner)
            let role_code = match payload.role {
                RoleEnum::Admin => 3,
                RoleEnum::Teacher => 2,
                _ => 0,
            };

            blockchain
                .assign_role(&wallet_address, role_code)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to assign role on blockchain (you need to be contract owner): {}", e),
                    )
                })?;
        }
    }

    // Create user-major relationships
    if let Some(major_ids) = payload.major_ids {
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
        wallet_address,
        encrypted_private_key,
        is_first_login: user.is_first_login,
        created_at: user.create_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/bulk",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Bulk user creation completed", body = BulkUserResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn create_users_bulk(
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<BulkUserResponse>), (StatusCode, String)> {
    // Get DB and blockchain from global state
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");
    let blockchain = get_admin_blockchain_service().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize blockchain service: {}", e),
        )
    })?;
    let mut file_data: Option<Vec<u8>> = None;

    // Extract file from multipart
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to read multipart: {}", e),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to read file: {}", e),
                )
            })?;
            file_data = Some(data.to_vec());
            break;
        }
    }

    let file_data =
        file_data.ok_or_else(|| (StatusCode::BAD_REQUEST, "No file provided".to_string()))?;

    // Parse Excel file
    let cursor = Cursor::new(file_data);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to open Excel file: {}", e),
        )
    })?;

    let sheet_names = workbook.sheet_names().to_owned();
    let first_sheet = sheet_names.first().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Excel file has no sheets".to_string(),
        )
    })?;

    let range = workbook.worksheet_range(first_sheet).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to read sheet: {}", e),
        )
    })?;

    let mut users_data: Vec<ExcelUserRow> = Vec::new();
    let mut errors: Vec<BulkUserError> = Vec::new();

    // Parse rows (skip header row)
    for (idx, row) in range.rows().enumerate().skip(1) {
        let row_num = idx + 1;

        let parse_result: Result<ExcelUserRow, String> = (|| {
            let get_cell = |col: usize| -> Result<String, String> {
                row.get(col)
                    .ok_or_else(|| format!("Missing column {}", col))?
                    .as_string()
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("Invalid data in column {}", col))
            };

            let user_row = ExcelUserRow {
                first_name: get_cell(0)?,
                last_name: get_cell(1)?,
                address: get_cell(2)?,
                email: get_cell(3)?,
                password: get_cell(4)?,
                cccd: get_cell(5)?,
                phone_number: get_cell(6)?,
                role: get_cell(7)?,
                student_code: row
                    .get(8)
                    .and_then(|cell| cell.as_string())
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty()),
            };

            user_row.validate()?;
            Ok(user_row)
        })();

        match parse_result {
            Ok(user_row) => users_data.push(user_row),
            Err(error) => {
                errors.push(BulkUserError {
                    row: row_num,
                    email: row
                        .get(3)
                        .and_then(|c| c.as_string())
                        .unwrap_or("unknown".to_string())
                        .to_string(),
                    error,
                });
            }
        }
    }

    let total_records = users_data.len() + errors.len();
    let mut successful = 0;

    // Prepare data for batch blockchain registration
    let mut student_addresses = Vec::new();
    let mut student_codes = Vec::new();
    let mut student_names = Vec::new();
    let mut student_emails = Vec::new();

    // Process each user
    for user_data in users_data.iter() {
        // Generate wallet
        let (wallet_address, wallet_private_key) = match BlockchainService::generate_wallet() {
            Ok(wallet) => wallet,
            Err(e) => {
                errors.push(BulkUserError {
                    row: 0,
                    email: user_data.email.clone(),
                    error: format!("Failed to generate wallet: {}", e),
                });
                continue;
            }
        };

        let user_id = Uuid::new_v4();
        let wallet_id = Uuid::new_v4();
        let now = Utc::now().naive_utc();

        let hashed_password = match bcrypt::hash(&user_data.password, bcrypt::DEFAULT_COST) {
            Ok(hash) => hash,
            Err(e) => {
                errors.push(BulkUserError {
                    row: 0,
                    email: user_data.email.clone(),
                    error: format!("Failed to hash password: {}", e),
                });
                continue;
            }
        };

        let role = match user_data.parse_role() {
            Ok(r) => r,
            Err(e) => {
                errors.push(BulkUserError {
                    row: 0,
                    email: user_data.email.clone(),
                    error: e,
                });
                continue;
            }
        };

        // Insert user into database
        let user_model = user::ActiveModel {
            user_id: Set(user_id),
            first_name: Set(user_data.first_name.clone()),
            last_name: Set(user_data.last_name.clone()),
            address: Set(user_data.address.clone()),
            email: Set(user_data.email.clone()),
            password: Set(hashed_password),
            is_priority: Set(false),
            cccd: Set(user_data.cccd.clone()),
            phone_number: Set(user_data.phone_number.clone()),
            is_first_login: Set(true),
            create_at: Set(now),
            update_at: Set(now),
            role: Set(role.clone()),
        };

        if let Err(e) = user_model.insert(db).await {
            errors.push(BulkUserError {
                row: 0,
                email: user_data.email.clone(),
                error: format!("Failed to create user: {}", e),
            });
            continue;
        }
        
        let encrypted_private_key = match encrypt_private_key(&wallet_private_key, &APP_CONFIG.encryption_key) {
            Ok(encrypted) => encrypted,
            Err(e) => {
                errors.push(BulkUserError {
                    row: 0,
                    email: user_data.email.clone(),
                    error: format!("Failed to encrypt private key: {}", e),
                });
                continue;
            }
        };
        
        let wallet_model = wallet::ActiveModel {
            wallet_id: Set(wallet_id),
            user_id: Set(user_id),
            address: Set(wallet_address.clone()),
            private_key: Set(encrypted_private_key),
            chain_type: Set("ethereum".to_string()),
            public_key: Set(wallet_address.clone()),
            status: Set("active".to_string()),
            network_id: Set("1".to_string()),
            last_used_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        if let Err(e) = wallet_model.insert(db).await {
            errors.push(BulkUserError {
                row: 0,
                email: user_data.email.clone(),
                error: format!("Failed to create wallet: {}", e),
            });
            continue;
        }

        // Collect student data for batch registration
        if role == RoleEnum::Student {
            if let Some(student_code) = &user_data.student_code {
                student_addresses.push(wallet_address.clone());
                student_codes.push(student_code.clone());
                student_names.push(format!("{} {}", user_data.first_name, user_data.last_name));
                student_emails.push(user_data.email.clone());
            }
        }

        successful += 1;
    }

    // Batch register students on blockchain (max 50 at a time)
    if !student_addresses.is_empty() {
        for chunk in 0..(student_addresses.len() + 49) / 50 {
            let start = chunk * 50;
            let end = std::cmp::min(start + 50, student_addresses.len());

            if let Err(e) = blockchain
                .register_students_batch(
                    student_addresses[start..end].to_vec(),
                    student_codes[start..end].to_vec(),
                    student_names[start..end].to_vec(),
                    student_emails[start..end].to_vec(),
                )
                .await
            {
                tracing::error!("Failed to register batch on blockchain: {}", e);
                // Don't fail the entire operation, just log the error
            }
        }
    }

    let response = BulkUserResponse {
        total_records,
        successful,
        failed: errors.len(),
        errors,
    };

    Ok((StatusCode::CREATED, Json(response)))
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
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Check permission: Admin or Manager
    permission::is_admin_or_manager(&auth_claims)?;

    let mut query = user::Entity::find();

    // If manager, only show students
    if auth_claims.role == UserRole::MANAGER {
        query = query.filter(user::Column::Role.eq(RoleEnum::Student));
    }

    // Filter by role if provided
    if let Some(role) = &params.role {
        query = query.filter(user::Column::Role.eq(role.clone()));
    }

    // Search by name or email
    if let Some(search) = &params.search {
        let search_pattern = format!("%{}%", search);
        query = query.filter(
            user::Column::FirstName
                .contains(&search_pattern)
                .or(user::Column::LastName.contains(&search_pattern))
                .or(user::Column::Email.contains(&search_pattern)),
        );
    }

    // Get total count
    let total = query
        .clone()
        .count(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    // Apply pagination
    let offset = (params.page - 1) * params.page_size;
    let users = query
        .order_by_desc(user::Column::CreateAt)
        .limit(params.page_size as u64)
        .offset(offset as u64)
        .all(db)
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
        let wallet_info = wallet::Entity::find()
            .filter(wallet::Column::UserId.eq(user_model.user_id))
            .one(db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                )
            })?;

        // Get major IDs
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
            .collect();

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
            created_at: user_model.create_at,
            updated_at: user_model.update_at,
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
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Get target user
    let target_user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Check permission
    let user_id_str = user_id.to_string();
    if auth_claims.role != UserRole::ADMIN {
        // Manager can only see students
        if auth_claims.role == UserRole::MANAGER && target_user.role != RoleEnum::Student {
            return Err((
                StatusCode::FORBIDDEN,
                "Managers can only view student accounts".to_string(),
            ));
        }
        // Non-admin/manager can only see themselves
        if auth_claims.role != UserRole::MANAGER
            && auth_claims.user_id != user_id_str
        {
            return Err((
                StatusCode::FORBIDDEN,
                "You can only view your own profile".to_string(),
            ));
        }
    }

    // Get wallet info
    let wallet_info = wallet::Entity::find()
        .filter(wallet::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    // Get major IDs
    let major_relationships = user_major::Entity::find()
        .filter(user_major::Column::UserId.eq(user_id))
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
        .collect();

    let response = UserDetailResponse {
        user_id: target_user.user_id,
        first_name: target_user.first_name,
        last_name: target_user.last_name,
        address: target_user.address,
        email: target_user.email,
        cccd: target_user.cccd,
        phone_number: target_user.phone_number,
        role: target_user.role,
        is_priority: target_user.is_priority,
        is_first_login: target_user.is_first_login,
        wallet_address: wallet_info.map(|w| w.address),
        major_ids,
        created_at: target_user.create_at,
        updated_at: target_user.update_at,
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
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Get target user
    let target_user = user::Entity::find_by_id(user_id)
        .one(db)
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

    let now = Utc::now().naive_utc();
    let mut active_user: user::ActiveModel = target_user.into();

    // Update fields if provided
    if let Some(first_name) = payload.first_name {
        active_user.first_name = Set(first_name);
    }
    if let Some(last_name) = payload.last_name {
        active_user.last_name = Set(last_name);
    }
    if let Some(address) = payload.address {
        active_user.address = Set(address);
    }
    if let Some(email) = payload.email {
        active_user.email = Set(email);
    }
    if let Some(password) = payload.password {
        let hashed = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to hash password: {}", e),
            )
        })?;
        active_user.password = Set(hashed);
    }
    if let Some(cccd) = payload.cccd {
        active_user.cccd = Set(cccd);
    }
    if let Some(phone_number) = payload.phone_number {
        active_user.phone_number = Set(phone_number);
    }
    if let Some(role) = payload.role {
        // Only admin can change roles
        if auth_claims.role != UserRole::ADMIN {
            return Err((
                StatusCode::FORBIDDEN,
                "Only admin can change user roles".to_string(),
            ));
        }
        active_user.role = Set(role);
    }

    active_user.update_at = Set(now);

    let updated_user = active_user.update(db).await.map_err(|e| {
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
    let wallet_info = wallet::Entity::find()
        .filter(wallet::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    let major_relationships = user_major::Entity::find()
        .filter(user_major::Column::UserId.eq(user_id))
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
        .collect();

    let response = UserDetailResponse {
        user_id: updated_user.user_id,
        first_name: updated_user.first_name,
        last_name: updated_user.last_name,
        address: updated_user.address,
        email: updated_user.email,
        cccd: updated_user.cccd,
        phone_number: updated_user.phone_number,
        role: updated_user.role,
        is_priority: updated_user.is_priority,
        is_first_login: updated_user.is_first_login,
        wallet_address: wallet_info.map(|w| w.address),
        major_ids,
        created_at: updated_user.create_at,
        updated_at: updated_user.update_at,
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
    let db = DATABASE_CONNECTION
        .get()
        .expect("DATABASE_CONNECTION not set");

    // Get target user
    let target_user = user::Entity::find_by_id(user_id)
        .one(db)
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

    // Delete user major relationships first (foreign key constraint)
    user_major::Entity::delete_many()
        .filter(user_major::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete user majors: {}", e),
            )
        })?;

    // Delete wallet
    wallet::Entity::delete_many()
        .filter(wallet::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete wallet: {}", e),
            )
        })?;

    // Delete user
    user::Entity::delete_by_id(user_id)
        .exec(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete user: {}", e),
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

