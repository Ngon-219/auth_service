use auth_service::rabbitmq_service::consumers;
use auth_service::rabbitmq_service::consumers::{
    RABBITMQ_CONNECTION, RabbitMqConsumer, get_rabbitmq_connetion,
};
use auth_service::rabbitmq_service::rabbitmq_service::RabbitMQService;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    get_rabbitmq_connetion().await;
    let rabbitmq_connection = RABBITMQ_CONNECTION
        .get()
        .expect("Failed to get rabbitmq connection");
    RabbitMQService::create_register_new_user_channel(rabbitmq_connection)
        .await
        .ok();

    RabbitMqConsumer::consume_register_new_student().await?;

    Ok(())
}
