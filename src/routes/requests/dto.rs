use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::entities::sea_orm_active_enums::RequestStatusEnum;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRequestRequest {
    pub content: String,
    pub authenticator_code: Option<String>, // Required if MFA is enabled
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestResponse {
    pub request_id: String,
    pub user_id: String,
    pub content: String,
    pub status: RequestStatusEnum,
    pub scheduled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleRequestRequest {
    pub scheduled_at: String, // Format: YYYY-MM-DDTHH:MM:SS
    pub message: Option<String>, // Optional message to include in email
    pub authenticator_code: Option<String>, // Required if MFA is enabled
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleRequestResponse {
    pub success: bool,
    pub message: String,
    pub request: RequestResponse,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestListResponse {
    pub requests: Vec<RequestResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestQueryParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    pub status: Option<RequestStatusEnum>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

