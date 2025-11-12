use crate::config::APP_CONFIG;
use crate::rabbitmq_service::consumers::REGISTER_NEW_USER_CHANNEL;
use crate::rabbitmq_service::structs::RegisterNewUserMessage;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*};
use serde_json::json;

pub struct RabbitMQService;

impl RabbitMQService {
    pub async fn new() -> Connection {
        let connection =
            Connection::connect(&APP_CONFIG.rabbitmq_uri, ConnectionProperties::default())
                .await
                .expect("Failed to connect to RabbitMQ");
        connection
    }

    pub async fn create_mail_queue(connection: Connection) -> Result<(), anyhow::Error> {
        let channel = connection
            .create_channel()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ channel: {}", e))?;

        channel
            .queue_declare(
                "mail_service",
                QueueDeclareOptions::default(),
                Default::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ queue: {}", e))?;

        Ok(())
    }

    pub async fn create_register_new_user_channel(
        connection: &Connection,
    ) -> Result<(), anyhow::Error> {
        let channel = connection
            .create_channel()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ channel: {}", e))?;

        channel
            .queue_declare(
                REGISTER_NEW_USER_CHANNEL,
                QueueDeclareOptions::default(),
                Default::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ queue: {}", e))?;

        Ok(())
    }

    pub async fn publish_to_mail_queue(
        connection: Connection,
        to: &str,
        subject: &str,
        email_data: &str,
    ) -> Result<(), anyhow::Error> {
        let standard_msg = json!({
            "pattern": "send-email",
            "data": {
                "to": to,
                "subject": subject,
                "text": email_data
            }
        });

        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                "mail_service",
                BasicPublishOptions::default(),
                standard_msg.to_string().as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_register_new_user(
        connection: &Connection,
        message: RegisterNewUserMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;

        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                REGISTER_NEW_USER_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }
}
