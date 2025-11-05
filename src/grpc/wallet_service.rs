use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::blockchain::helpers::get_user_private_key;
use crate::static_service::get_database_connection;

pub mod wallet {
    tonic::include_proto!("wallet");
}

use wallet::{
    ExportPrivateKeyRequest, ExportPrivateKeyResponse,
    wallet_service_server::{WalletService, WalletServiceServer},
};

pub struct WalletServiceImpl;

#[tonic::async_trait]
impl WalletService for WalletServiceImpl {
    async fn export_private_key(
        &self,
        request: Request<ExportPrivateKeyRequest>,
    ) -> Result<Response<ExportPrivateKeyResponse>, Status> {
        let req = request.into_inner();

        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid user_id: {}", e)))?;

        let db = get_database_connection().await;

        match get_user_private_key(&db, &user_id).await {
            Ok(private_key) => Ok(Response::new(ExportPrivateKeyResponse {
                success: true,
                private_key,
                message: "Private key exported successfully".to_string(),
            })),
            Err(e) => {
                tracing::error!("Failed to export private key for user {}: {}", user_id, e);
                Err(Status::not_found(format!(
                    "Failed to export private key: {}",
                    e
                )))
            }
        }
    }
}

pub fn create_wallet_service() -> WalletServiceServer<WalletServiceImpl> {
    WalletServiceServer::new(WalletServiceImpl)
}
