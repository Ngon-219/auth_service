use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,

    #[schema(example = "password123")]
    pub password: String,

    #[schema(example = "123456")]
    pub authenticator_code: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user_id: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ForgotPasswordResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    #[schema(example = "user@example.com")]
    pub email: String,

    #[schema(example = "12345678")]
    pub otp_code: String,

    #[schema(example = "newPassword123")]
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResetPasswordResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[schema(example = "oldPassword123")]
    pub old_password: String,

    #[schema(example = "newPassword123")]
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChangePasswordResponse {
    pub message: String,
}
