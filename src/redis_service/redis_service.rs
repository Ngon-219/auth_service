use crate::config::{
    APP_CONFIG, FILE_TRACKER_EXPRIED_TIME, JWT_EXPRIED_TIME, MFA_CODE_REUSE_TTL_SECONDS,
    MFA_LOCK_DURATION_SECONDS, MFA_MAX_FAIL_ATTEMPTS,
};
use anyhow::{Context, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

pub static REDIS_CLIENT: Lazy<redis::Client> = Lazy::new(|| {
    redis::Client::open(APP_CONFIG.redis_url.as_str()).expect("Failed to create Redis client")
});

pub async fn init_redis_connection() -> Result<()> {
    // Test connection
    let mut conn = REDIS_CLIENT
        .get_connection_manager()
        .await
        .context("Failed to get Redis connection")?;

    let _: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .context("Failed to ping Redis")?;

    Ok(())
}

pub async fn get_redis() -> Result<ConnectionManager> {
    REDIS_CLIENT
        .get_connection_manager()
        .await
        .context("Failed to get Redis connection")
}

// MFA attempts stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaAttempts {
    pub user_id: String,
    pub invalid_mfa_count: u32,
    pub locked_until: Option<i64>, // Unix timestamp
}

impl MfaAttempts {
    /// Check if MFA is currently locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            let now = Utc::now().timestamp();
            now < locked_until
        } else {
            false
        }
    }

    /// Increment fail count and set locked_until if needed
    pub fn increment_fail(&mut self) {
        self.invalid_mfa_count += 1;
        if self.invalid_mfa_count >= MFA_MAX_FAIL_ATTEMPTS {
            let now = Utc::now().timestamp();
            self.locked_until = Some(now + MFA_LOCK_DURATION_SECONDS as i64);
        }
    }

    /// Reset fail count and lock
    pub fn reset(&mut self) {
        self.invalid_mfa_count = 0;
        self.locked_until = None;
    }
}

// MFA-related Redis operations
pub struct MfaRedisService;

impl MfaRedisService {
    /// Get MFA attempts for a user
    pub async fn get_mfa_attempts(user_id: &str) -> Result<MfaAttempts> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;
        let key = format!("mfa:attempts:{}", user_id);

        match redis.get::<_, Option<String>>(&key).await? {
            Some(json) => {
                let mfa_attempts: MfaAttempts =
                    serde_json::from_str(&json).context("Failed to deserialize MFA attempts")?;
                Ok(mfa_attempts)
            }
            None => {
                // Return default value if key doesn't exist
                Ok(MfaAttempts {
                    user_id: user_id.to_string(),
                    invalid_mfa_count: 0,
                    locked_until: None,
                })
            }
        }
    }

    /// Set MFA attempts for a user
    pub async fn set_mfa_attempts(user_id: &str, mfa_attempts: &MfaAttempts) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;
        let key = format!("mfa:attempts:{}", user_id);

        let json =
            serde_json::to_string(mfa_attempts).context("Failed to serialize MFA attempts")?;

        let _: () = redis.set_ex(&key, json, MFA_LOCK_DURATION_SECONDS).await?;
        Ok(())
    }

    /// Reset MFA attempts for a user
    pub async fn reset_mfa_attempts(user_id: &str) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;
        let key = format!("mfa:attempts:{}", user_id);
        let _: () = redis.del(&key).await?;
        Ok(())
    }

    /// Check if a code has been used before
    pub async fn is_mfa_code_used(user_id: &str, code: &str) -> Result<bool> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;
        let key = format!("mfa:used_code:{}:{}", user_id, code);
        let exists: bool = redis.exists(&key).await?;
        Ok(exists)
    }

    /// Mark a code as used (with TTL)
    pub async fn mark_mfa_code_as_used(user_id: &str, code: &str) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;
        let key = format!("mfa:used_code:{}:{}", user_id, code);
        let now = Utc::now().timestamp();
        let _: () = redis.set_ex(&key, now, MFA_CODE_REUSE_TTL_SECONDS).await?;
        Ok(())
    }
}

pub struct JwtBlacklist;

impl JwtBlacklist {
    pub async fn add_jwt_to_blacklist(user_id: &str, jwt: &str) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("jwt:blacklist:{}:{}", user_id, jwt);
        let now = Utc::now().timestamp();
        let _: () = redis.set_ex(&key, now, JWT_EXPRIED_TIME as u64).await?;
        Ok(())
    }

    pub async fn check_jwt_in_blacklist(user_id: &str, jwt: &str) -> Result<bool> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("jwt:blacklist:{}:{}", user_id, jwt);
        let exists: bool = redis.exists(&key).await?;
        Ok(exists)
    }
}

pub struct FileHandleTrackProgress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProgress {
    pub total: u64,
    pub current: u64,
    pub percent: u64,
    pub success: u64,
    pub failed: u64,
}

impl FileHandleTrackProgress {
    pub async fn set_total_file_handle(file_name: &str, total: u64) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("file_handle_tracker:{}", file_name);
        let _: () = redis
            .set_ex(&key, total, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(())
    }

    pub async fn set_current_file_progress(file_name: &str, current: u64) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("file_handle_progress:{}", file_name);

        if current == 0 {
            let _: () = redis
                .set_ex(&key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
                .await?;
            return Ok(());
        }

        let existing: Option<u64> = redis
            .get(&key)
            .await
            .context("Failed to read current file progress from Redis")?;
        let next_value = existing.map(|value| value.max(current)).unwrap_or(current);

        let _: () = redis
            .set_ex(&key, next_value, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(())
    }

    pub async fn reset_success_failed(file_name: &str) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let success_key = format!("file_handle_success:{}", file_name);
        let failed_key = format!("file_handle_failed:{}", file_name);

        let _: () = redis
            .set_ex(&success_key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;
        let _: () = redis
            .set_ex(&failed_key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(())
    }

    pub async fn increment_success(file_name: &str) -> Result<()> {
        increment_counter(
            &format!("file_handle_success:{}", file_name),
            FILE_TRACKER_EXPRIED_TIME as u64,
        )
        .await
    }

    pub async fn increment_failed(file_name: &str) -> Result<()> {
        increment_counter(
            &format!("file_handle_failed:{}", file_name),
            FILE_TRACKER_EXPRIED_TIME as u64,
        )
        .await
    }
}

pub async fn helper_get_current_file_progress(file_name: &str) -> Result<FileProgress> {
    let mut redis = get_redis()
        .await
        .context("Failed to get Redis connection")?;

    let key_total_file_handle = format!("file_handle_tracker:{}", file_name);
    let key_current_file_handle = format!("file_handle_progress:{}", file_name);
    let key_success = format!("file_handle_success:{}", file_name);
    let key_failed = format!("file_handle_failed:{}", file_name);

    let total_file_handle: Option<u64> = redis
        .get(&key_total_file_handle)
        .await
        .context("Failed to get total file handle from Redis")?;

    let current_file_handle: Option<u64> = redis
        .get(&key_current_file_handle)
        .await
        .context("Failed to get current file handle from Redis")?;

    let total = total_file_handle.unwrap_or(0);
    let mut current = current_file_handle.unwrap_or(0);
    if total > 0 && current > total {
        current = total;
    }

    let percent = if total == 0 {
        0
    } else {
        current.saturating_mul(100).checked_div(total).unwrap_or(0)
    };

    let success: Option<u64> = redis
        .get(&key_success)
        .await
        .context("Failed to get success count from Redis")?;
    let failed: Option<u64> = redis
        .get(&key_failed)
        .await
        .context("Failed to get failed count from Redis")?;

    Ok(FileProgress {
        total,
        current,
        percent,
        success: success.unwrap_or(0),
        failed: failed.unwrap_or(0),
    })
}

pub struct BlockchainRegistrationProgress;

impl BlockchainRegistrationProgress {
    pub async fn set_total(
        file_upload_history_id: &str,
        total: u64,
    ) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("blockchain_registration_tracker:{}", file_upload_history_id);
        let _: () = redis
            .set_ex(&key, total, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(())
    }

    pub async fn reset_progress(
        file_upload_history_id: &str,
    ) -> Result<()> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!(
            "blockchain_registration_progress:{}",
            file_upload_history_id
        );

        let _: () = redis
            .set_ex(&key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        let success_key = format!(
            "blockchain_registration_success:{}",
            file_upload_history_id
        );
        let failed_key = format!(
            "blockchain_registration_failed:{}",
            file_upload_history_id
        );

        let _: () = redis
            .set_ex(&success_key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;
        let _: () = redis
            .set_ex(&failed_key, 0u64, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(())
    }

    async fn increment(
        key: &str,
        ttl: u64,
    ) -> Result<u64> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let current: Option<u64> = redis.get(key).await?;
        let next_value = current.map(|v| v + 1).unwrap_or(1);

        let _: () = redis
            .set_ex(key, next_value, ttl)
            .await?;

        Ok(next_value)
    }

    async fn increment_progress_internal(
        file_upload_history_id: &str,
    ) -> Result<u64> {
        let mut redis = get_redis()
            .await
            .context("Failed to get Redis connection")?;

        let key = format!("blockchain_registration_progress:{}", file_upload_history_id);

        let current: Option<u64> = redis.get(&key).await?;
        let next_value = current.map(|v| v + 1).unwrap_or(1);

        let _: () = redis
            .set_ex(&key, next_value, FILE_TRACKER_EXPRIED_TIME as u64)
            .await?;

        Ok(next_value)
    }

    pub async fn increment_success(
        file_upload_history_id: &str,
    ) -> Result<()> {
        Self::increment_progress_internal(file_upload_history_id).await?;
        Self::increment(
            &format!(
                "blockchain_registration_success:{}",
                file_upload_history_id
            ),
            FILE_TRACKER_EXPRIED_TIME as u64,
        )
        .await?;
        Ok(())
    }

    pub async fn increment_failed(
        file_upload_history_id: &str,
    ) -> Result<()> {
        Self::increment_progress_internal(file_upload_history_id).await?;
        Self::increment(
            &format!(
                "blockchain_registration_failed:{}",
                file_upload_history_id
            ),
            FILE_TRACKER_EXPRIED_TIME as u64,
        )
        .await?;
        Ok(())
    }
}

pub async fn helper_get_blockchain_registration_progress(
    file_upload_history_id: &str,
) -> Result<FileProgress> {
    let mut redis = get_redis()
        .await
        .context("Failed to get Redis connection")?;

    let key_total = format!("blockchain_registration_tracker:{}", file_upload_history_id);
    let key_current = format!("blockchain_registration_progress:{}", file_upload_history_id);
    let key_success = format!("blockchain_registration_success:{}", file_upload_history_id);
    let key_failed = format!("blockchain_registration_failed:{}", file_upload_history_id);

    let total: Option<u64> = redis
        .get(&key_total)
        .await
        .context("Failed to get total blockchain registration from Redis")?;

    let current: Option<u64> = redis
        .get(&key_current)
        .await
        .context("Failed to get current blockchain registration from Redis")?;

    let total = total.unwrap_or(0);
    let mut current = current.unwrap_or(0);
    if total > 0 && current > total {
        current = total;
    }

    let percent = if total == 0 {
        0
    } else {
        current.saturating_mul(100).checked_div(total).unwrap_or(0)
    };

    let success: Option<u64> = redis
        .get(&key_success)
        .await
        .context("Failed to get success blockchain registration from Redis")?;
    let failed: Option<u64> = redis
        .get(&key_failed)
        .await
        .context("Failed to get failed blockchain registration from Redis")?;

    Ok(FileProgress {
        total,
        current,
        percent,
        success: success.unwrap_or(0),
        failed: failed.unwrap_or(0),
    })
}

async fn increment_counter(key: &str, ttl: u64) -> Result<()> {
    let mut redis = get_redis()
        .await
        .context("Failed to get Redis connection")?;

    let current: Option<u64> = redis.get(key).await?;
    let next_value = current.map(|v| v + 1).unwrap_or(1);

    let _: () = redis.set_ex(key, next_value, ttl).await?;
    Ok(())
}
