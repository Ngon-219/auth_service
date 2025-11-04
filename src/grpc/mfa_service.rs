use tonic::{Request, Response, Status};

use crate::repositories::UserMfaRepository;

pub mod mfa {
    tonic::include_proto!("mfa");
}

use mfa::{
    mfa_service_server::{MfaService, MfaServiceServer},
    VerifyMfaCodeRequest, VerifyMfaCodeResponse,
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

        match mfa_repo.verify_mfa_code(&req.user_id, &req.authenticator_code).await {
            Ok(is_valid) => Ok(Response::new(VerifyMfaCodeResponse {
                is_valid,
            })),
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

