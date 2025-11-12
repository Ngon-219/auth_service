use crate::config::{APP_CONFIG, MFA_MAX_FAIL_ATTEMPTS};
use crate::entities::user_mfa;
use crate::redis_service::redis_service::MfaRedisService;
use crate::repositories::mfa_verify_result::MfaVerifyResult;
use crate::static_service::DATABASE_CONNECTION;
use crate::utils::encryption::decrypt;
use anyhow::Result;
use google_authenticator::GoogleAuthenticator;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

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

    pub async fn find_by_user_id(&self, user_id: Uuid) -> Result<Option<user_mfa::Model>> {
        let db = self.get_connection();
        let mfa = user_mfa::Entity::find()
            .filter(user_mfa::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        Ok(mfa)
    }

    pub async fn find_enabled_by_user_id(&self, user_id: Uuid) -> Result<Option<user_mfa::Model>> {
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
        let mfa = self
            .find_by_user_id(user_id)
            .await?
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
        let mfa = self
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("MFA record not found"))?;
        let db = self.get_connection();

        let mut mfa_model: user_mfa::ActiveModel = mfa.into();
        mfa_model.backup_codes = Set(backup_codes);
        mfa_model.updated_at = Set(chrono::Utc::now().naive_utc());

        let result = mfa_model.update(db).await?;
        Ok(result)
    }

    pub async fn verify_mfa_code(
        &self,
        user_id: &str,
        code: &str,
    ) -> anyhow::Result<MfaVerifyResult> {
        // Get MFA attempts
        let mut mfa_attempts = MfaRedisService::get_mfa_attempts(user_id).await?;

        // Check if MFA is locked
        if mfa_attempts.is_locked() {
            tracing::warn!(
                "MFA is locked for user {} due to too many failed attempts",
                user_id
            );
            return Ok(MfaVerifyResult::Locked {
                locked_until: mfa_attempts.locked_until,
            });
        }

        // Check if code has been used before
        if MfaRedisService::is_mfa_code_used(user_id, code).await? {
            tracing::warn!("MFA code {} already used for user {}", code, user_id);
            return Ok(MfaVerifyResult::CodeAlreadyUsed);
        }

        let user_mfa = self.find_enabled_by_user_id(user_id.parse()?).await?;

        let Some(user_mfa) = user_mfa else {
            return Ok(MfaVerifyResult::MfaNotEnabled);
        };

        let decrypted_secret = decrypt(&APP_CONFIG.encryption_key, &user_mfa.secret)
            .map_err(|e| anyhow::anyhow!("Failed to decrypt secret: {}", e))?;

        let auth = GoogleAuthenticator::new();

        let is_valid = auth.verify_code(&decrypted_secret, code, 1, 0);

        if is_valid {
            // Mark code as used
            MfaRedisService::mark_mfa_code_as_used(user_id, code).await?;

            // Reset fail count and lock on success
            MfaRedisService::reset_mfa_attempts(user_id).await?;

            tracing::info!("MFA code verified successfully for user {}", user_id);
            Ok(MfaVerifyResult::Success)
        } else {
            // Increment fail count (will set locked_until if >= 3)
            mfa_attempts.increment_fail();
            let locked_until = mfa_attempts.locked_until;
            MfaRedisService::set_mfa_attempts(user_id, &mfa_attempts).await?;

            tracing::warn!(
                "MFA code verification failed for user {} (attempt {})",
                user_id,
                mfa_attempts.invalid_mfa_count
            );

            if mfa_attempts.invalid_mfa_count >= MFA_MAX_FAIL_ATTEMPTS {
                tracing::error!(
                    "MFA locked for user {} for 15 minutes due to {} failed attempts",
                    user_id,
                    MFA_MAX_FAIL_ATTEMPTS
                );
                Ok(MfaVerifyResult::Locked { locked_until })
            } else {
                Ok(MfaVerifyResult::InvalidCode)
            }
        }
    }
}
