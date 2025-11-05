use std::fs;
use std::net::SocketAddr;

use auth_service::bootstrap::initialize_admin_user;
use auth_service::grpc::start_grpc_server;
use auth_service::rabbitmq_service::rabbitmq_service::RabbitMQService;
use auth_service::redis_service::init_redis_connection;
use auth_service::static_service::get_database_connection;
use auth_service::{app, config::APP_CONFIG, utils::tracing::init_standard_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    init_standard_tracing(env!("CARGO_CRATE_NAME"));

    tracing::info!("Starting application...");

    tracing::info!("Create upload folder");
    fs::create_dir_all("./uploads/temp").unwrap();

    // Initialize database connection
    let db_connection = get_database_connection().await;

    let rabbit_mq = RabbitMQService::new().await;

    if let Ok(()) = RabbitMQService::create_mail_queue(rabbit_mq).await {
        tracing::info!("Create rabbitmq queue successfully");
    }

    // Initialize Redis connection
    tracing::info!("Initializing Redis connection...");
    if let Err(e) = init_redis_connection().await {
        tracing::error!("Failed to initialize Redis connection: {}", e);
        tracing::warn!("Continuing without Redis (MFA features may not work properly)...");
    } else {
        tracing::info!("Redis connection initialized successfully");
    }

    // Initialize default admin user
    tracing::info!("Checking admin user...");
    if let Err(e) = initialize_admin_user(db_connection).await {
        tracing::error!("Failed to initialize admin user: {}", e);
        tracing::warn!("Continuing without admin user initialization...");
    }

    eprintln!("🚀 About to create app...");
    let app = app::create_app().await?;
    eprintln!("✅ App created successfully!");

    let http_address = format!("0.0.0.0:{}", APP_CONFIG.port);

    // Start gRPC server in background
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = start_grpc_server().await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    tracing::info!("HTTP server listening on {}", &http_address);
    tracing::info!("gRPC server listening on 0.0.0.0:{}", APP_CONFIG.grpc_port);

    let listener = tokio::net::TcpListener::bind(http_address).await.unwrap();

    // Run HTTP server
    let http_result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;

    // Cancel gRPC server if HTTP server stops
    grpc_handle.abort();

    http_result.expect("Failed to start HTTP server");

    Ok(())
}
