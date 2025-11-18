use crate::config::APP_CONFIG;
use crate::rabbitmq_service::consumers::{
    ACTIVATE_STUDENT_CHANNEL, ASSIGN_ROLE_CHANNEL, CREATE_USER_DB, DEACTIVATE_STUDENT_CHANNEL,
    REGISTER_NEW_MANAGER_CHANNEL, REGISTER_NEW_USER_CHANNEL, REGISTER_STUDENTS_BATCH_CHANNEL,
    REMOVE_MANAGER_CHANNEL,
};
use crate::rabbitmq_service::structs::{
    ActivateStudentMessage, AssignRoleMessage, DeactivateStudentMessage, RegisterNewManagerMessage,
    RegisterNewUserMessage, RegisterStudentsBatchMessage, RemoveManagerMessage,
};
use crate::routes::users::dto::UserCsvColumn;
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

    pub async fn create_mail_queue(connection: &Connection) -> Result<(), anyhow::Error> {
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

    pub async fn create_user_db_channel(connection: &Connection) -> Result<(), anyhow::Error> {
        let channel = connection
            .create_channel()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ channel: {}", e))?;

        channel
            .queue_declare(
                CREATE_USER_DB,
                QueueDeclareOptions::default(),
                Default::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ queue: {}", e))?;

        Ok(())
    }

    pub async fn create_register_new_manager_channel(
        connection: &Connection,
    ) -> Result<(), anyhow::Error> {
        let channel = connection
            .create_channel()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ channel: {}", e))?;

        channel
            .queue_declare(
                REGISTER_NEW_MANAGER_CHANNEL,
                QueueDeclareOptions::default(),
                Default::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ queue: {}", e))?;

        Ok(())
    }

    pub async fn create_all_blockchain_queues(
        connection: &Connection,
    ) -> Result<(), anyhow::Error> {
        let channel = connection
            .create_channel()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ channel: {}", e))?;

        let queues = [
            ASSIGN_ROLE_CHANNEL,
            REMOVE_MANAGER_CHANNEL,
            DEACTIVATE_STUDENT_CHANNEL,
            ACTIVATE_STUDENT_CHANNEL,
            REGISTER_STUDENTS_BATCH_CHANNEL,
        ];

        for queue in queues.iter() {
            channel
                .queue_declare(queue, QueueDeclareOptions::default(), Default::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create RabbitMQ queue {}: {}", queue, e))?;
        }

        Ok(())
    }

    pub async fn publish_to_mail_queue(
        connection: &Connection,
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

    pub async fn publish_to_register_new_manager(
        connection: &Connection,
        message: RegisterNewManagerMessage,
    ) -> Result<(), anyhow::Error> {
        let seriablize_msg = serde_json::to_string(&message)?;

        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                REGISTER_NEW_MANAGER_CHANNEL,
                BasicPublishOptions::default(),
                seriablize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_assign_role(
        connection: &Connection,
        message: AssignRoleMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;
        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                ASSIGN_ROLE_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_remove_manager(
        connection: &Connection,
        message: RemoveManagerMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;
        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                REMOVE_MANAGER_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_deactivate_student(
        connection: &Connection,
        message: DeactivateStudentMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;
        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                DEACTIVATE_STUDENT_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_activate_student(
        connection: &Connection,
        message: ActivateStudentMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;
        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                ACTIVATE_STUDENT_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_register_students_batch(
        connection: &Connection,
        message: RegisterStudentsBatchMessage,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;
        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                REGISTER_STUDENTS_BATCH_CHANNEL,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn publish_to_create_user_db(
        connection: &Connection,
        message: UserCsvColumn,
    ) -> Result<(), anyhow::Error> {
        let serialize_msg = serde_json::to_string(&message)?;

        let channel = connection.create_channel().await?;

        channel
            .basic_publish(
                "",
                CREATE_USER_DB,
                BasicPublishOptions::default(),
                serialize_msg.as_bytes(),
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }
}
