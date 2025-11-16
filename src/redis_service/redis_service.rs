use crate::config::{
    APP_CONFIG, JWT_EXPRIED_TIME, MFA_CODE_REUSE_TTL_SECONDS, MFA_LOCK_DURATION_SECONDS,
    MFA_MAX_FAIL_ATTEMPTS,
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
