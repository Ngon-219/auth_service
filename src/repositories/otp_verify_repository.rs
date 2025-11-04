use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, QueryOrder, QuerySelect, ActiveModelTrait, Set};
use uuid::Uuid;
use crate::entities::otp_verify;
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};

pub struct OtpVerifyRepository;

impl OtpVerifyRepository {
    pub fn new() -> Self {
        Self
    }

    fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        otp_code: String,
        email: String,
        purpose: String,
        expires_at: NaiveDateTime,
    ) -> Result<otp_verify::Model> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();
        let otp_model = otp_verify::ActiveModel {
            otp_id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            otp_code: Set(otp_code),
            email: Set(email),
            purpose: Set(purpose),
            is_verified: Set(false),
            expires_at: Set(expires_at),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = otp_model.insert(db).await?;
        Ok(result)
    }

    pub async fn find_latest_by_user_and_purpose(
        &self,
        user_id: Uuid,
        purpose: &str,
    ) -> Result<Option<otp_verify::Model>> {
        let db = self.get_connection();
        let otp = otp_verify::Entity::find()
            .filter(otp_verify::Column::UserId.eq(user_id))
            .filter(otp_verify::Column::Purpose.eq(purpose))
            .order_by_desc(otp_verify::Column::CreatedAt)
            .limit(1)
            .one(db)
            .await?;
        Ok(otp)
    }

    pub async fn mark_as_verified(
        &self,
        otp_id: Uuid,
    ) -> Result<otp_verify::Model> {
        let db = self.get_connection();
        let otp = otp_verify::Entity::find_by_id(otp_id)
            .one(db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OTP not found"))?;

        let mut otp_model: otp_verify::ActiveModel = otp.into();
        otp_model.is_verified = Set(true);
        otp_model.updated_at = Set(Utc::now().naive_utc());

        let result = otp_model.update(db).await?;
        Ok(result)
    }

    pub async fn find_by_id(
        &self,
        otp_id: Uuid,
    ) -> Result<Option<otp_verify::Model>> {
        let db = self.get_connection();
        let otp = otp_verify::Entity::find_by_id(otp_id)
            .one(db)
            .await?;
        Ok(otp)
    }
}
