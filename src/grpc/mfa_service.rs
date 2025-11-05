use tonic::{Request, Response, Status};

use crate::repositories::UserMfaRepository;

pub mod mfa {
    tonic::include_proto!("mfa");
}

use mfa::{
    VerifyMfaCodeRequest, VerifyMfaCodeResponse,
    mfa_service_server::{MfaService, MfaServiceServer},
};

pub struct MfaServiceImpl;

#[tonic::async_trait]
impl MfaService for MfaServiceImpl {
    async fn verify_mfa_code(
        &self,
        request: Request<VerifyMfaCodeRequest>,
    ) -> Result<Response<VerifyMfaCodeResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() {
            return Err(Status::invalid_argument("user_id is required"));
        }

        if req.authenticator_code.is_empty() {
            return Err(Status::invalid_argument("authenticator_code is required"));
        }

        let mfa_repo = UserMfaRepository::new();

        use crate::repositories::mfa_verify_result::MfaVerifyResult;

        match mfa_repo
            .verify_mfa_code(&req.user_id, &req.authenticator_code)
            .await
        {
            Ok(result) => {
                let (is_valid, reason, message, locked_until) = match result {
                    MfaVerifyResult::Success => (
                        true,
                        "success".to_string(),
                        "MFA code verified successfully".to_string(),
                        0i64,
                    ),
                    MfaVerifyResult::Locked { locked_until } => (
                        false,
                        "locked".to_string(),
                        format!(
                            "MFA is locked{}",
                            locked_until
                                .map(|u| format!(" until {}", u))
                                .unwrap_or_default()
                        ),
                        locked_until.unwrap_or(0),
                    ),
                    MfaVerifyResult::CodeAlreadyUsed => (
                        false,
                        "code_already_used".to_string(),
                        "MFA code has already been used".to_string(),
                        0,
                    ),
                    MfaVerifyResult::InvalidCode => (
                        false,
                        "invalid_code".to_string(),
                        "Invalid MFA code".to_string(),
                        0,
                    ),
                    MfaVerifyResult::MfaNotEnabled => (
                        false,
                        "mfa_not_enabled".to_string(),
                        "MFA is not enabled for this user".to_string(),
                        0,
                    ),
                };

                Ok(Response::new(VerifyMfaCodeResponse {
                    is_valid,
                    reason,
                    message,
                    locked_until: if locked_until > 0 { locked_until } else { 0 },
                }))
            }
            Err(e) => {
                tracing::error!("Failed to verify MFA code for user {}: {}", req.user_id, e);
                Err(Status::internal(format!(
                    "Failed to verify MFA code: {}",
                    e
                )))
            }
        }
    }
}

pub fn create_mfa_service() -> MfaServiceServer<MfaServiceImpl> {
    MfaServiceServer::new(MfaServiceImpl)
}
