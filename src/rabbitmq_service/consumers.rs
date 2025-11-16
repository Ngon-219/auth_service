use crate::blockchain::BlockchainService;
use crate::config::APP_CONFIG;
use crate::rabbitmq_service::structs::{
    ActivateStudentMessage, AssignRoleMessage, DeactivateStudentMessage,
    RegisterNewManagerMessage, RegisterNewUserMessage, RegisterStudentsBatchMessage,
    RemoveManagerMessage,
};
use crate::redis_service::redis_emitter::RedisEmitter;
use crate::repositories::UserRepository;
use anyhow::Context;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties};
use serde_json::json;
use tokio::sync::OnceCell;

pub const REGISTER_NEW_USER_CHANNEL: &str = "create::new::user";
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
        tracing::info!("Starting consumer for register new student queue: {}", REGISTER_NEW_USER_CHANNEL);
        
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
                Ok(payload) => {
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
                        tracing::debug!("Message acknowledged, starting blockchain registration...");
                        
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
                                let notification = json!({
                                    "status": "success",
                                    "student_code": deserialize_payload.student_code,
                                    "email": deserialize_payload.email,
                                    "message": "Register student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.email),
                                    &notification,
                                )
                                .await;

                                tracing::info!(
                                    "Successfully registered student {} on blockchain and sent notification",
                                    deserialize_payload.student_code
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to register new student: {}", e);
                                let user_repo = UserRepository::new();
                                tracing::info!(
                                    "Attempting to delete user with student_code: {} after blockchain registration failed",
                                    deserialize_payload.student_code
                                );
                                if let Err(delete_err) = user_repo
                                    .delete_by_student_code(&deserialize_payload.student_code)
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to delete user after blockchain error: {}",
                                        delete_err
                                    );
                                } else {
                                    tracing::info!(
                                        "Successfully deleted user with student_code: {} after blockchain registration failed",
                                        deserialize_payload.student_code
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
                                    &format!("user:{}", deserialize_payload.email),
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
        tracing::info!("Starting consumer for register new manager queue: {}", REGISTER_NEW_MANAGER_CHANNEL);

        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        tracing::info!("Created RabbitMQ channel, starting to consume messages...");

        let mut consumer = channel
            .basic_consume(
                REGISTER_NEW_MANAGER_CHANNEL ,
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
                Ok(payload) => {
                    let deserialize_payload: RegisterNewManagerMessage =
                        serde_json::from_slice::<RegisterNewManagerMessage>(&delivery.data)?;

                    tracing::info!(
                        "Processing register manager message for address: {}",
                        deserialize_payload.wallet_address
                    );

                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge register new manager message: {}", e);
                    } else {
                        tracing::debug!("Message acknowledged, starting blockchain registration...");

                        let blockchain =
                            BlockchainService::new(&deserialize_payload.private_key).await?;
                        let result = blockchain
                            .add_manager(
                                &deserialize_payload.wallet_address,
                            )
                            .await;

                        match result {
                            Ok(_) => {
                                let notification = json!({
                                    "status": "success",
                                    "email": deserialize_payload.email,
                                    "message": "Register manager on blockchain successfully."
                                })
                                    .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", deserialize_payload.email),
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
                                let user_repo = UserRepository::new();
                                tracing::info!(
                                    "Attempting to delete user with email: {} after blockchain registration failed",
                                    deserialize_payload.email
                                );
                                if let Err(delete_err) = user_repo
                                    .delete_by_email(&deserialize_payload.email)
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to delete user after blockchain error: {}",
                                        delete_err
                                    );
                                } else {
                                    tracing::info!(
                                        "Successfully deleted user with email: {} after blockchain registration failed",
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
                                    &format!("user:{}", deserialize_payload.email),
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
        tracing::info!("Starting consumer for assign role queue: {}", ASSIGN_ROLE_CHANNEL);
        
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
                        let blockchain =
                            BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .assign_role(&payload.user_address, payload.role)
                            .await;

                        match result {
                            Ok(_) => {
                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Assign role on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to assign role on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
        tracing::info!("Starting consumer for remove manager queue: {}", REMOVE_MANAGER_CHANNEL);
        
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
                        let blockchain =
                            BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .remove_manager(&payload.manager_address)
                            .await;

                        match result {
                            Ok(_) => {
                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Remove manager from blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to remove manager from blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
        tracing::info!("Starting consumer for deactivate student queue: {}", DEACTIVATE_STUDENT_CHANNEL);
        
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
                        let blockchain =
                            BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .deactivate_student(payload.student_id)
                            .await;

                        match result {
                            Ok(_) => {
                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Deactivate student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to deactivate student on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
        tracing::info!("Starting consumer for activate student queue: {}", ACTIVATE_STUDENT_CHANNEL);
        
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
                        let blockchain =
                            BlockchainService::new(&payload.private_key).await?;
                        let result = blockchain
                            .activate_student(payload.student_id)
                            .await;

                        match result {
                            Ok(_) => {
                                let notification = json!({
                                    "status": "success",
                                    "email": payload.email,
                                    "message": "Activate student on blockchain successfully."
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
                                let notification = json!({
                                    "status": "failed",
                                    "email": payload.email,
                                    "reason": e.to_string(),
                                    "message": "Failed to activate student on blockchain. Please try again or contact with admin"
                                })
                                .to_string();

                                RedisEmitter::emit_to_rooom(
                                    &format!("user:{}", payload.email),
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
        tracing::info!("Starting consumer for register students batch queue: {}", REGISTER_STUDENTS_BATCH_CHANNEL);
        
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
                        tracing::error!("Failed to acknowledge register students batch message: {}", e);
                    } else {
                        let blockchain =
                            BlockchainService::new(&payload.private_key).await?;
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
                                // Send notification to all students
                                for email in payload.emails.iter() {
                                    let notification = json!({
                                        "status": "success",
                                        "email": email,
                                        "message": "Batch register students on blockchain successfully."
                                    })
                                    .to_string();

                                    RedisEmitter::emit_to_rooom(
                                        &format!("user:{}", email),
                                        &notification,
                                    )
                                    .await;
                                }

                                tracing::info!(
                                    "Successfully registered {} students in batch and sent notifications",
                                    payload.wallet_addresses.len()
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to register students batch: {}", e);
                                // Send failure notification to all students
                                for email in payload.emails.iter() {
                                    let notification = json!({
                                        "status": "failed",
                                        "email": email,
                                        "reason": e.to_string(),
                                        "message": "Failed to register students batch on blockchain. Please try again or contact with admin"
                                    })
                                    .to_string();

                                    RedisEmitter::emit_to_rooom(
                                        &format!("user:{}", email),
                                        &notification,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to deserialize register students batch message: {}", e);
                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        }

        Ok(())
    }
}