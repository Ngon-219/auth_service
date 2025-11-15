use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::APP_CONFIG;

pub fn init_standard_tracing(crate_name: &str) {
    let level = &APP_CONFIG.log_level;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // Include auth_service module explicitly for HTTP logger
                format!("{crate_name}={level},auth_service::middleware={level},tower_http={level},api_wallet_evm={level},test_kafka={level}").into()
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_file(false)
                .with_line_number(false)
        )
        .init();
}
