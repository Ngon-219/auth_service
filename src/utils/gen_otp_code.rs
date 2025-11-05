use crate::utils::random::generate_random_string;
use chrono::{DateTime, Duration, Utc};

pub fn gen_code() -> anyhow::Result<(String, DateTime<Utc>)> {
    const EXPIRES_IN_MINUTES: i64 = 10; // 10 minutes
    const TOKEN_LENGTH: usize = 8;

    let token = generate_random_string(TOKEN_LENGTH);

    let now = Utc::now();
    let expires_at = now + Duration::minutes(EXPIRES_IN_MINUTES);

    Ok((token, expires_at))
}
