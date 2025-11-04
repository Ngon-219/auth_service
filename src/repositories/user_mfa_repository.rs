use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set};
use uuid::Uuid;
use crate::entities::user_mfa;
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;

pub struct UserMfaRepository;

impl UserMfaRepository {
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
    ) -> Result<Option<user_mfa::Model>> {
        let db = self.get_connection();
        let mfa = user_mfa::Entity::find()
            .filter(user_mfa::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        Ok(mfa)
    }

    pub async fn find_enabled_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<user_mfa::Model>> {
        let db = self.get_connection();
        let mfa = user_mfa::Entity::find()
            .filter(user_mfa::Column::UserId.eq(user_id))
            .filter(user_mfa::Column::IsEnabled.eq(true))
            .one(db)
            .await?;
        Ok(mfa)
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        secret: String,
        backup_codes: Option<String>,
    ) -> Result<user_mfa::Model> {
        let db = self.get_connection();
        let now = chrono::Utc::now().naive_utc();
        let mfa_model = user_mfa::ActiveModel {
            mfa_id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            secret: Set(secret),
            is_enabled: Set(true),
            backup_codes: Set(backup_codes),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = mfa_model.insert(db).await?;
        Ok(result)
    }

    pub async fn update_enabled_status(
        &self,
        user_id: Uuid,
        is_enabled: bool,
    ) -> Result<user_mfa::Model> {
        let mfa = self.find_by_user_id(user_id).await?
            .ok_or_else(|| anyhow::anyhow!("MFA record not found"))?;
        let db = self.get_connection();

        let mut mfa_model: user_mfa::ActiveModel = mfa.into();
        mfa_model.is_enabled = Set(is_enabled);
        mfa_model.updated_at = Set(chrono::Utc::now().naive_utc());

        let result = mfa_model.update(db).await?;
        Ok(result)
    }

    pub async fn update_backup_codes(
        &self,
        user_id: Uuid,
        backup_codes: Option<String>,
    ) -> Result<user_mfa::Model> {
        let mfa = self.find_by_user_id(user_id).await?
            .ok_or_else(|| anyhow::anyhow!("MFA record not found"))?;
        let db = self.get_connection();

        let mut mfa_model: user_mfa::ActiveModel = mfa.into();
        mfa_model.backup_codes = Set(backup_codes);
        mfa_model.updated_at = Set(chrono::Utc::now().naive_utc());

        let result = mfa_model.update(db).await?;
        Ok(result)
    }
}
