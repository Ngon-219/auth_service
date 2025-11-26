use crate::blockchain::BlockchainService;
use crate::config::APP_CONFIG;
use crate::entities::sea_orm_active_enums::{RoleEnum, UserStatus};
use crate::rabbitmq_service::structs::{
    ActivateStudentMessage, AssignRoleMessage, DeactivateStudentMessage, RegisterNewManagerMessage,
    RegisterNewUserMessage, RegisterStudentsBatchMessage, RemoveManagerMessage,
};
use crate::redis_service::redis_emitter::RedisEmitter;
use crate::redis_service::redis_service::{
    BlockchainRegistrationProgress, FileHandleTrackProgress,
};
use crate::repositories::{UserRepository, WalletRepository};
use crate::routes::users::dto::UserCsvColumn;
use crate::utils::encryption::encrypt_private_key;
use anyhow::{Context, anyhow};
use chrono::Utc;
use futures::StreamExt;
use http::StatusCode;
use lapin::options::{BasicAckOptions, BasicConsumeOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;
use tokio::sync::OnceCell;
use uuid::Uuid;
use crate::entities::user_major;

pub const REGISTER_NEW_USER_CHANNEL: &str = "create::new::user";
pub const CREATE_USER_DB: &str = "create::user::db";
pub const REGISTER_NEW_MANAGER_CHANNEL: &str = "create:new:manager";
pub const ASSIGN_ROLE_CHANNEL: &str = "blockchain::assign::role";
pub const REMOVE_MANAGER_CHANNEL: &str = "blockchain::remove::manager";
pub const DEACTIVATE_STUDENT_CHANNEL: &str = "blockchain::deactivate::student";
pub const ACTIVATE_STUDENT_CHANNEL: &str = "blockchain::activate::student";
pub const REGISTER_STUDENTS_BATCH_CHANNEL: &str = "blockchain::register::students::batch";

pub static RABBITMQ_CONNECTION: OnceCell<Connection> = OnceCell::const_new();

pub async fn get_rabbitmq_connetion() -> &'static Connection {
    RABBITMQ_CONNECTION
        .get_or_init(|| async {
            Connection::connect(&APP_CONFIG.rabbitmq_uri, ConnectionProperties::default())
                .await
                .expect("Failed to connect to RabbitMQ")
        })
        .await
}
pub struct RabbitMqConsumer;

impl RabbitMqConsumer {
    pub async fn new() -> Connection {
        let connection =
            Connection::connect(&APP_CONFIG.rabbitmq_uri, ConnectionProperties::default())
                .await
                .expect("Failed to connect to RabbitMQ");
        connection
    }

    pub async fn consume_register_new_student() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for register new student queue: {}",
            REGISTER_NEW_USER_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        tracing::info!("Created RabbitMQ channel, starting to consume messages...");

        let mut consumer = channel
            .basic_consume(
                REGISTER_NEW_USER_CHANNEL,
                "register_student",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            tracing::debug!("Received message from queue");
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match std::str::from_utf8(&delivery.data) {
                Ok(_payload) => {
                    let deserialize_payload: RegisterNewUserMessage =
                        serde_json::from_slice::<RegisterNewUserMessage>(&delivery.data)?;

                    tracing::info!(
                        "Processing register student message for student_code: {}, email: {}",
                        deserialize_payload.student_code,
                        deserialize_payload.email
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge register new user message: {}", e);
                    } else {
                        tracing::debug!(
                            "Message acknowledged, starting blockchain registration..."
                        );

                        let blockchain =
                            BlockchainService::new(&deserialize_payload.private_key).await?;
                        let result = blockchain
                            .register_student(
                                &deserialize_payload.wallet_address,
                                &deserialize_payload.student_code,
                                &deserialize_payload.full_name,
                                &deserialize_payload.email,
                            )
                            .await;

                        match result {
                            Ok(_) => {
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &deserialize_payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        deserialize_payload.email,
                                        status_err
                                    );
                                }

                                // Update progress if file_upload_history_id is present
                                if let Some(file_upload_history_id) =
                                    deserialize_payload.file_upload_history_id.as_deref()
                                {
                                    if let Err(progress_err) =
                                        BlockchainRegistrationProgress::increment_success(
                                            file_upload_history_id,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to update blockchain registration progress for {}: {}",
                                            file_upload_history_id,
                                            progress_err
                                        );
                                    }
                                }

                                let notification = json!({
                                    "status": "success",
                                    "student_code": deserialize_payload.student_code,
                                    "email": deserialize_payload.email,
                                    "message": "Register student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully registered student {} on blockchain and sent notification",
                                    deserialize_payload.student_code
                                );
                            }
                            Err(e) => {
                                // Update progress even on failure (to track total processed)
                                if let Some(file_upload_history_id) =
                                    deserialize_payload.file_upload_history_id.as_deref()
                                {
                                    if let Err(progress_err) =
                                        BlockchainRegistrationProgress::increment_failed(
                                            file_upload_history_id,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to update blockchain registration progress for {}: {}",
                                            file_upload_history_id,
                                            progress_err
                                        );
                                    }
                                }

                                tracing::error!("Failed to register new student: {}", e);
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &deserialize_payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        deserialize_payload.email,
                                        status_err
                                    );
                                } else {
                                    tracing::info!(
                                        "Set user status to Failed for {} after blockchain error",
                                        deserialize_payload.email
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "student_code": deserialize_payload.student_code,
                                    "email": deserialize_payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to register student on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Sent failure notification for student {} after blockchain registration failed",
                                    deserialize_payload.student_code
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to consumer message rabbitmq: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_register_new_manager() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for register new manager queue: {}",
            REGISTER_NEW_MANAGER_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        tracing::info!("Created RabbitMQ channel, starting to consume messages...");

        let mut consumer = channel
            .basic_consume(
                REGISTER_NEW_MANAGER_CHANNEL,
                "register_manager",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            tracing::debug!("Received message from queue");
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match std::str::from_utf8(&delivery.data) {
                Ok(_payload) => {
                    let deserialize_payload: RegisterNewManagerMessage =
                        serde_json::from_slice::<RegisterNewManagerMessage>(&delivery.data)?;

                    tracing::info!(
                        "Processing register manager message for address: {}",
                        deserialize_payload.wallet_address
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!(
                            "Failed to acknowledge register new manager message: {}",
                            e
                        );
                    } else {
                        tracing::debug!(
                            "Message acknowledged, starting blockchain registration..."
                        );

                        let blockchain =
                            BlockchainService::new(&deserialize_payload.private_key).await?;
                        let result = blockchain
                            .add_manager(&deserialize_payload.wallet_address)
                            .await;

                        match result {
                            Ok(_) => {
                                // Update user status to Sync after successful blockchain registration
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &deserialize_payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        deserialize_payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "success",
                                    "email": deserialize_payload.email,
                                    "message": "Register manager on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully registered new manager {} on blockchain and sent notification",
                                    deserialize_payload.email
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to register new manager: {}", e);
                                
                                // Update user status to Failed after blockchain error
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &deserialize_payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        deserialize_payload.email,
                                        status_err
                                    );
                                } else {
                                    tracing::info!(
                                        "Set user status to Failed for {} after blockchain error",
                                        deserialize_payload.email
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "email": deserialize_payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to register manager on blockchain. Please try again or contact with admin"
                                })
                                    .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Sent failure notification for new manager {} after blockchain registration failed",
                                    deserialize_payload.email
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to consumer message rabbitmq: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_assign_role() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for assign role queue: {}",
            ASSIGN_ROLE_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .ok_or_else(|| anyhow::anyhow!("RabbitMQ connection not initialized"))?;

        let channel = rabbit_conn
            .create_channel()
            .await
            .context("Failed to create RabbitMQ channel")?;

        let mut consumer = channel
            .basic_consume(
                ASSIGN_ROLE_CHANNEL,
                "assign_role",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to start consuming from queue")?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<AssignRoleMessage>(&delivery.data) {
                Ok(payload) => {
                    tracing::info!(
                        "Processing assign role message for address: {}, role: {}",
                        payload.user_address,
                        payload.role
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge assign role message: {}", e);
                    } else {
                        let blockchain = BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .assign_role(&payload.user_address, payload.role)
                            .await;

                        match result {
                            Ok(_) => {
                                // Update user status to Sync after successful blockchain role assignment
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Assign role on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully assigned role {} to {} and sent notification",
                                    payload.role,
                                    payload.user_address
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to assign role: {}", e);
                                
                                // Update user status to Failed after blockchain error
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to assign role on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize assign role message: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_remove_manager() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for remove manager queue: {}",
            REMOVE_MANAGER_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .ok_or_else(|| anyhow::anyhow!("RabbitMQ connection not initialized"))?;

        let channel = rabbit_conn
            .create_channel()
            .await
            .context("Failed to create RabbitMQ channel")?;

        let mut consumer = channel
            .basic_consume(
                REMOVE_MANAGER_CHANNEL,
                "remove_manager",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to start consuming from queue")?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<RemoveManagerMessage>(&delivery.data) {
                Ok(payload) => {
                    tracing::info!(
                        "Processing remove manager message for address: {}",
                        payload.manager_address
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge remove manager message: {}", e);
                    } else {
                        let blockchain = BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain.remove_manager(&payload.manager_address).await;

                        match result {
                            Ok(_) => {
                                // Update user status to Sync after successful blockchain manager removal
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Remove manager from blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully removed manager {} and sent notification",
                                    payload.manager_address
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to remove manager: {}", e);
                                
                                // Update user status to Failed after blockchain error
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to remove manager from blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize remove manager message: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_deactivate_student() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for deactivate student queue: {}",
            DEACTIVATE_STUDENT_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .ok_or_else(|| anyhow::anyhow!("RabbitMQ connection not initialized"))?;

        let channel = rabbit_conn
            .create_channel()
            .await
            .context("Failed to create RabbitMQ channel")?;

        let mut consumer = channel
            .basic_consume(
                DEACTIVATE_STUDENT_CHANNEL,
                "deactivate_student",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to start consuming from queue")?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<DeactivateStudentMessage>(&delivery.data) {
                Ok(payload) => {
                    tracing::info!(
                        "Processing deactivate student message for student_id: {}",
                        payload.student_id
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge deactivate student message: {}", e);
                    } else {
                        let blockchain = BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain.deactivate_student(payload.student_id).await;

                        match result {
                            Ok(_) => {
                                // Update user status to Sync after successful blockchain deactivation
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Deactivate student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully deactivated student {} and sent notification",
                                    payload.student_id
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to deactivate student: {}", e);
                                
                                // Update user status to Failed after blockchain error
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to deactivate student on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize deactivate student message: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_activate_student() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for activate student queue: {}",
            ACTIVATE_STUDENT_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .ok_or_else(|| anyhow::anyhow!("RabbitMQ connection not initialized"))?;

        let channel = rabbit_conn
            .create_channel()
            .await
            .context("Failed to create RabbitMQ channel")?;

        let mut consumer = channel
            .basic_consume(
                ACTIVATE_STUDENT_CHANNEL,
                "activate_student",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to start consuming from queue")?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<ActivateStudentMessage>(&delivery.data) {
                Ok(payload) => {
                    tracing::info!(
                        "Processing activate student message for student_id: {}",
                        payload.student_id
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge activate student message: {}", e);
                    } else {
                        let blockchain = BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain.activate_student(payload.student_id).await;

                        match result {
                            Ok(_) => {
                                // Update user status to Sync after successful blockchain activation
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Sync,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Sync for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Activate student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully activated student {} and sent notification",
                                    payload.student_id
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to activate student: {}", e);
                                
                                // Update user status to Failed after blockchain error
                                let user_repo = UserRepository::new();
                                if let Err(status_err) = user_repo
                                    .update_status_by_email(
                                        &payload.email,
                                        UserStatus::Failed,
                                    )
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to update user status to Failed for {}: {}",
                                        payload.email,
                                        status_err
                                    );
                                }

                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to activate student on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize activate student message: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consume_register_students_batch() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for register students batch queue: {}",
            REGISTER_STUDENTS_BATCH_CHANNEL
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .ok_or_else(|| anyhow::anyhow!("RabbitMQ connection not initialized"))?;

        let channel = rabbit_conn
            .create_channel()
            .await
            .context("Failed to create RabbitMQ channel")?;

        let mut consumer = channel
            .basic_consume(
                REGISTER_STUDENTS_BATCH_CHANNEL,
                "register_students_batch",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .context("Failed to start consuming from queue")?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<RegisterStudentsBatchMessage>(&delivery.data) {
                Ok(payload) => {
                    tracing::info!(
                        "Processing register students batch message for {} students",
                        payload.wallet_addresses.len()
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!(
                            "Failed to acknowledge register students batch message: {}",
                            e
                        );
                    } else {
                        let blockchain = BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .register_students_batch(
                                payload.wallet_addresses.clone(),
                                payload.student_codes.clone(),
                                payload.full_names.clone(),
                                payload.emails.clone(),
                            )
                            .await;

                        match result {
                            Ok(_) => {
                                // Update status to Sync for all students after successful batch registration
                                let user_repo = UserRepository::new();
                                for email in &payload.emails {
                                    if let Err(status_err) = user_repo
                                        .update_status_by_email(email, UserStatus::Sync)
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to update user status to Sync for {}: {}",
                                            email,
                                            status_err
                                        );
                                    }
                                }

                                // Send notification to creator
                                let notification = json!({
                                    "status": "success",
                                    "total_students": payload.emails.len(),
                                    "message": "Batch register students on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully registered {} students in batch and sent notifications",
                                    payload.wallet_addresses.len()
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to register students batch: {}", e);
                                
                                // Update status to Failed for all students after batch registration error
                                let user_repo = UserRepository::new();
                                for email in &payload.emails {
                                    if let Err(status_err) = user_repo
                                        .update_status_by_email(email, UserStatus::Failed)
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to update user status to Failed for {}: {}",
                                            email,
                                            status_err
                                        );
                                    }
                                }

                                // Send failure notification to creator
                                let notification = json!({
                                    "status": "failed",
                                    "total_students": payload.emails.len(),
                                    "reason": e.to_string(),
                                    "message": "Failed to register students batch on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.creator_user_id),
                                    &notification,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to deserialize register students batch message: {}",
                        e
                    );
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn consumer_create_user_db() -> Result<(), anyhow::Error> {
        tracing::info!(
            "Starting consumer for create user db queue: {}",
            CREATE_USER_DB
        );

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        tracing::info!("Created RabbitMQ channel, starting to consume messages...");

        let mut consumer = channel
            .basic_consume(
                CREATE_USER_DB,
                "create_user_db",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!("Consumer started successfully, waiting for messages...");

        while let Some(delivery) = consumer.next().await {
            tracing::debug!("Received message from queue");
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to receive message rabbitMQ: {:?}", e);
                    continue;
                }
            };

            match std::str::from_utf8(&delivery.data) {
                Ok(_payload) => {
                    let deserialize_payload: UserCsvColumn =
                        serde_json::from_slice::<UserCsvColumn>(&delivery.data)?;

                    tracing::info!(
                        "Processing register student message for email: {}",
                        deserialize_payload.email,
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge register new user message: {}", e);
                    } else {
                        tracing::debug!("Message acknowledged, starting user create db...");

                        match Self::create_user_from_csv_payload(&deserialize_payload).await {
                            Ok(_) => {
                                if let Some(file_name) = deserialize_payload.file_name.as_deref() {
                                    if let Some(row_number) = deserialize_payload.row_number {
                                        if let Err(progress_err) =
                                            FileHandleTrackProgress::set_current_file_progress(
                                                file_name, row_number,
                                            )
                                            .await
                                        {
                                            tracing::error!(
                                                "Failed to update file progress for {}: {}",
                                                file_name,
                                                progress_err
                                            );
                                        }
                                    }

                                    if let Err(success_err) =
                                        FileHandleTrackProgress::increment_success(file_name).await
                                    {
                                        tracing::error!(
                                            "Failed to increment success counter for {}: {}",
                                            file_name,
                                            success_err
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::error!("Failed to create user from CSV payload: {err:?}");
                                if let Some(file_name) = deserialize_payload.file_name.as_deref() {
                                    if let Err(failed_err) =
                                        FileHandleTrackProgress::increment_failed(file_name).await
                                    {
                                        tracing::error!(
                                            "Failed to increment failed counter for {}: {}",
                                            file_name,
                                            failed_err
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to consumer message rabbitmq: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }
}

impl RabbitMqConsumer {
    async fn create_user_from_csv_payload(payload: &UserCsvColumn) -> anyhow::Result<()> {
        let user_repo = UserRepository::new();
        let wallet_repo = WalletRepository::new();

        let hashed_password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
            .map_err(|e| anyhow!("Failed to hash password: {e}"))?;

        let (wallet_address, wallet_private_key) =
            BlockchainService::generate_wallet().context("Failed to generate wallet")?;

        let encrypted_private_key =
            encrypt_private_key(&wallet_private_key, &APP_CONFIG.encryption_key)
                .map_err(|e| anyhow!("Failed to encrypt private key: {e}"))?;

        let user_id = Uuid::new_v4();
        let wallet_id = Uuid::new_v4();

        let student_code = if payload.role.to_string() == "Student".to_string() {
            let latest_student_code = UserRepository::get_latest_student_code()
                .await
                .unwrap_or_else(|_| "000000".into());
            let student_code_i64 = latest_student_code.parse::<i64>().unwrap_or_default();
            Some(format!("{:06}", student_code_i64 + 1))
        } else {
            None
        };

        let user_role = match payload.role.as_str() {
            "Student" => RoleEnum::Student,
            "Manager" => RoleEnum::Manager,
            "Admin" => RoleEnum::Admin,
            _ => {
                tracing::error!("Invalid role: {}", payload.role);
                return Err(anyhow!("Invalid role"));
            }
        };

        user_repo
            .create(
                user_id,
                payload.first_name.clone(),
                payload.last_name.clone(),
                payload.address.clone(),
                payload.email.clone(),
                hashed_password,
                payload.cccd.clone(),
                payload.phone_number.clone(),
                user_role,
                false,
                student_code.clone(),
            )
            .await
            .context("Failed to create user")?;

        wallet_repo
            .create(
                wallet_id,
                user_id,
                wallet_address.clone(),
                encrypted_private_key.clone(),
                APP_CONFIG.chain_type.clone(),
                wallet_address,
                "active".to_string(),
                APP_CONFIG.chain_id.clone(),
            )
            .await
            .context("Failed to create wallet")?;

        let db = user_repo.get_connection();
        let now = Utc::now().naive_utc();
        for major_id in payload.major_ids.iter() {
            let major_id_uuid = Uuid::parse_str(major_id).expect("Failed to parse major id");
            let relationship_model = user_major::ActiveModel {
                user_id: Set(user_id),
                major_id: Set(major_id_uuid),
                create_at: Set(now),
                updated_at: Set(now),
            };

            relationship_model.insert(db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create user-major relationship: {}", e),
                )
            })
                .map_err(|e| anyhow!("Failed to create user-major relationship: {}", e.1))?;
        }

        Ok(())
    }
}
