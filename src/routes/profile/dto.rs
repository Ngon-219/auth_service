use crate::entities::sea_orm_active_enums::{RoleEnum, UserStatus};
use chrono::NaiveDateTime;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProfileResponse {
    pub user: UserInfo,
    pub wallet: WalletInfo,
    pub majors: Vec<MajorInfo>,
    pub departments: Vec<DepartmentInfo>,
    pub blockchain_role: u8,
    pub is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub email: String,
    pub phone_number: String,
    pub cccd: String,
    pub is_priority: bool,
    pub is_first_login: bool,
    pub role: RoleEnum,
    pub status: UserStatus,
    pub student_code: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletInfo {
    pub wallet_id: Uuid,
    pub address: String,
    pub chain_type: String,
    pub public_key: String,
    pub status: String,
    pub network_id: String,
    pub last_used_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MajorInfo {
    pub major_id: Uuid,
    pub name: String,
    pub department_id: Option<Uuid>,
    pub founding_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DepartmentInfo {
    pub department_id: Uuid,
    pub name: String,
    pub dean: String,
    pub founding_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
