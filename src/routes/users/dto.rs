use crate::entities::sea_orm_active_enums::RoleEnum;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateUserRequest {
    #[schema(example = "Nguyen")]
    pub first_name: String,

    #[schema(example = "Van A")]
    pub last_name: String,

    #[schema(example = "123 Main St, Hanoi")]
    pub address: String,

    #[schema(example = "nguyenvana@example.com")]
    pub email: String,

    #[schema(example = "password123")]
    pub password: String,

    #[schema(example = "0123456789")]
    pub cccd: String,

    #[schema(example = "0912345678")]
    pub phone_number: String,

    #[schema(example = "student")]
    pub role: RoleEnum,

    #[serde(default)]
    pub major_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: RoleEnum,
    pub wallet_address: String,
    /// Private key of the generated wallet - ONLY returned on creation, store securely!
    pub wallet_private_key: String,
    pub is_first_login: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUserResponse {
    pub total_records: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<BulkUserError>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUserError {
    pub row: usize,
    pub email: String,
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct ExcelUserRow {
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub email: String,
    pub password: String,
    pub cccd: String,
    pub phone_number: String,
    pub role: String,
    pub student_code: Option<String>,
}

impl ExcelUserRow {
    pub fn validate(&self) -> Result<(), String> {
        if self.first_name.is_empty() {
            return Err("First name is required".to_string());
        }
        if self.last_name.is_empty() {
            return Err("Last name is required".to_string());
        }
        if self.email.is_empty() || !self.email.contains('@') {
            return Err("Valid email is required".to_string());
        }
        if self.password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }

        // Validate role
        match self.role.to_lowercase().as_str() {
            "student" | "teacher" | "admin" | "manager" => Ok(()),
            _ => Err(format!("Invalid role: {}", self.role)),
        }
    }

    pub fn parse_role(&self) -> Result<RoleEnum, String> {
        match self.role.to_lowercase().as_str() {
            "student" => Ok(RoleEnum::Student),
            "teacher" => Ok(RoleEnum::Teacher),
            "admin" => Ok(RoleEnum::Admin),
            "manager" => Ok(RoleEnum::Manager),
            _ => Err(format!("Invalid role: {}", self.role)),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserRequest {
    #[schema(example = "Nguyen")]
    pub first_name: Option<String>,

    #[schema(example = "Van A")]
    pub last_name: Option<String>,

    #[schema(example = "123 Main St, Hanoi")]
    pub address: Option<String>,

    #[schema(example = "nguyenvana@example.com")]
    pub email: Option<String>,

    /// New password (optional) - will be hashed
    #[schema(example = "newpassword123")]
    pub password: Option<String>,

    #[schema(example = "0123456789")]
    pub cccd: Option<String>,

    #[schema(example = "0912345678")]
    pub phone_number: Option<String>,

    #[schema(example = "student")]
    pub role: Option<RoleEnum>,

    /// Update major IDs (replaces existing)
    pub major_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDetailResponse {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub email: String,
    pub cccd: String,
    pub phone_number: String,
    pub role: RoleEnum,
    pub is_priority: bool,
    pub is_first_login: bool,
    pub wallet_address: Option<String>,
    pub major_ids: Vec<Uuid>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserDetailResponse>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct UserQueryParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub role: Option<RoleEnum>,
    pub search: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}
