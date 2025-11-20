use clap::Parser;
use once_cell::sync::Lazy;

pub const OTP_ISSUER: &str = "NGON";

// MFA Configuration
pub const MFA_MAX_FAIL_ATTEMPTS: u32 = 3;
pub const MFA_CODE_REUSE_TTL_SECONDS: u64 = 120; // 2 minutes
pub const MFA_LOCK_DURATION_SECONDS: u64 = 900; // 15 minutes
pub const JWT_EXPRIED_TIME: i64 = 86400i64;

pub const FILE_TRACKER_EXPRIED_TIME: i64 = 86400i64;

pub static APP_CONFIG: Lazy<Config> = Lazy::new(Config::parse);

#[derive(Debug, Parser, Clone)]
pub struct Config {
    #[clap(long, env, default_value_t = 8080)]
    pub port: u16,

    #[clap(long, env, default_value_t = true)]
    pub swagger_enabled: bool,

    #[clap(long, env)]
    pub log_level: String,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env)]
    pub blockchain_rpc_url: String,

    #[clap(long, env)]
    pub data_storage_contract_address: String,

    #[clap(long, env)]
    pub admin_private_key: String,

    #[clap(long, env)]
    pub encryption_key: String,

    #[clap(long, env, default_value_t = 50051)]
    pub grpc_port: u16,

    #[clap(long, env)]
    pub rabbitmq_uri: String,

    #[clap(long, env)]
    pub admin_email: String,

    #[clap(long, env)]
    pub admin_password: String,

    #[clap(long, env)]
    pub chain_type: String,

    #[clap(long, env)]
    pub chain_id: String,

    #[clap(long, env, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[clap(long, env, default_value = "local")]
    pub app_env: String,
}
