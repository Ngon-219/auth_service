use anyhow::{Context, Result};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::service::BlockchainService;
use crate::entities::wallet;
use crate::utils::encryption::decrypt_private_key;
use crate::config::APP_CONFIG;

pub async fn get_user_private_key(db: &DatabaseConnection, user_id: &Uuid) -> Result<String> {
    let wallet_info = wallet::Entity::find()
        .filter(wallet::Column::UserId.eq(*user_id))
        .one(db)
        .await
        .context("Failed to query wallet")?
        .ok_or_else(|| anyhow::anyhow!("Wallet not found for user"))?;
    
    decrypt_private_key(&wallet_info.private_key, &APP_CONFIG.encryption_key)
        .context("Failed to decrypt private key")
}

pub async fn get_user_blockchain_service(
    db: &DatabaseConnection,
    user_id: &Uuid,
) -> Result<BlockchainService> {
    let private_key = get_user_private_key(db, user_id).await?;
    BlockchainService::new(&private_key).await
}

pub async fn get_admin_blockchain_service() -> Result<BlockchainService> {
    use crate::config::APP_CONFIG;
    BlockchainService::new(&APP_CONFIG.admin_private_key).await
}
