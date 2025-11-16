use auth_service::rabbitmq_service::consumers::{
    RABBITMQ_CONNECTION, RabbitMqConsumer, get_rabbitmq_connetion,
};
use auth_service::rabbitmq_service::rabbitmq_service::RabbitMQService;
use auth_service::static_service::get_database_connection;
use auth_service::utils::tracing::init_standard_tracing;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    init_standard_tracing(env!("CARGO_CRATE_NAME"));
    tracing::info!("Initializing RabbitMQ connection...");

    get_rabbitmq_connetion().await;
    tracing::info!("RabbitMQ connection established");

    get_database_connection().await;
    tracing::info!("Database connection established");

    let rabbitmq_connection = RABBITMQ_CONNECTION
        .get()
        .expect("Failed to get rabbitmq connection");

    tracing::info!("Creating register new user channel...");
    RabbitMQService::create_register_new_user_channel(rabbitmq_connection)
        .await
        .ok();

    tracing::info!("Creating register new manager channel...");
    RabbitMQService::create_register_new_manager_channel(rabbitmq_connection)
        .await
        .ok();

    tracing::info!("Creating all blockchain queues...");
    RabbitMQService::create_all_blockchain_queues(rabbitmq_connection)
        .await
        .ok();

    tracing::info!("All queues created successfully");

    tracing::info!("Starting all consumers...");

    // Start all consumers in parallel
    let student_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting student consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_register_new_student().await {
            tracing::error!("Student consumer error: {:?}", e);
        }
    });

    let manager_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting manager consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_register_new_manager().await {
            tracing::error!("Manager consumer error: {:?}", e);
        }
    });

    let assign_role_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting assign role consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_assign_role().await {
            tracing::error!("Assign role consumer error: {:?}", e);
        }
    });

    let remove_manager_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting remove manager consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_remove_manager().await {
            tracing::error!("Remove manager consumer error: {:?}", e);
        }
    });

    let deactivate_student_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting deactivate student consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_deactivate_student().await {
            tracing::error!("Deactivate student consumer error: {:?}", e);
        }
    });

    let activate_student_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting activate student consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_activate_student().await {
            tracing::error!("Activate student consumer error: {:?}", e);
        }
    });

    let register_batch_consumer = tokio::spawn(async {
        tracing::info!("[Spawn] Starting register batch consumer task...");
        if let Err(e) = RabbitMqConsumer::consume_register_students_batch().await {
            tracing::error!("Register students batch consumer error: {:?}", e);
        }
    });

    // Give consumers a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    tracing::info!("All consumers started, waiting for messages...");

    // Wait for any consumer to stop (they run indefinitely)
    tokio::select! {
        _ = student_consumer => {
            tracing::warn!("Student consumer stopped");
        }
        _ = manager_consumer => {
            tracing::warn!("Manager consumer stopped");
        }
        _ = assign_role_consumer => {
            tracing::warn!("Assign role consumer stopped");
        }
        _ = remove_manager_consumer => {
            tracing::warn!("Remove manager consumer stopped");
        }
        _ = deactivate_student_consumer => {
            tracing::warn!("Deactivate student consumer stopped");
        }
        _ = activate_student_consumer => {
            tracing::warn!("Activate student consumer stopped");
        }
        _ = register_batch_consumer => {
            tracing::warn!("Register students batch consumer stopped");
        }
    }

    Ok(())
}
