use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;
use anyhow::Context;

use crate::config::APP_CONFIG;
use super::wallet_service::create_wallet_service;
use super::mfa_service::create_mfa_service;

pub async fn start_grpc_server() -> anyhow::Result<()> {
    let addr: SocketAddr = format!("0.0.0.0:{}", APP_CONFIG.grpc_port)
        .parse()
        .context("Invalid gRPC server address")?;

    let wallet_service = create_wallet_service();
    let mfa_service = create_mfa_service();

    info!("Starting gRPC server on {}", addr);

    Server::builder()
        .add_service(wallet_service)
        .add_service(mfa_service)
        .serve(addr)
        .await
        .context("gRPC server error")?;

    Ok(())
}

