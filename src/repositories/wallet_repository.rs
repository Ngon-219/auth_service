use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set, DeleteResult};
use uuid::Uuid;
use crate::entities::wallet;
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;

pub struct WalletRepository;

impl WalletRepository {
    pub fn new() -> Self {
        Self
    }

    fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<wallet::Model>> {
        let db = self.get_connection();
        let wallet_info = wallet::Entity::find()
            .filter(wallet::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        Ok(wallet_info)
    }

    pub async fn find_by_address(
        &self,
        address: &str,
    ) -> Result<Option<wallet::Model>> {
        let db = self.get_connection();
        let wallet_info = wallet::Entity::find()
            .filter(wallet::Column::Address.eq(address))
            .one(db)
            .await?;
        Ok(wallet_info)
    }

    pub async fn create(
        &self,
        wallet_id: Uuid,
        user_id: Uuid,
        address: String,
        private_key: String,
        chain_type: String,
        public_key: String,
        status: String,
        network_id: String,
    ) -> Result<wallet::Model> {
        let db = self.get_connection();
        let now = chrono::Utc::now().naive_utc();
        let wallet_model = wallet::ActiveModel {
            wallet_id: Set(wallet_id),
            user_id: Set(user_id),
            address: Set(address.clone()),
            private_key: Set(private_key),
            chain_type: Set(chain_type),
            public_key: Set(public_key),
            status: Set(status),
            network_id: Set(network_id),
            last_used_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = wallet_model.insert(db).await?;
        Ok(result)
    }

    pub async fn delete_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<DeleteResult> {
        let db = self.get_connection();
        let result = wallet::Entity::delete_many()
            .filter(wallet::Column::UserId.eq(user_id))
            .exec(db)
            .await?;
        Ok(result)
    }
}
