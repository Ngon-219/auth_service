use crate::blockchain::BlockchainService;
use crate::config::APP_CONFIG;
use crate::rabbitmq_service::structs::RegisterNewUserMessage;
use crate::repositories::UserRepository;
use futures::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties};
use tokio::sync::OnceCell;

pub const REGISTER_NEW_USER_CHANNEL: &str = "create::new::user";

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
        let rabbit_conn = RABBITMQ_CONNECTION
            .get()
            .expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        let mut consumer = channel
            .basic_consume(
                REGISTER_NEW_USER_CHANNEL,
                "register_student",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(delivery) = consumer.next().await {
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
                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        tracing::error!("Failed to acknowledge register new user message: {}", e);
                    } else {
                        let blockchain =
                            BlockchainService::new(&deserialize_payload.private_key).await?;
                        blockchain
                            .register_student(
                                &deserialize_payload.wallet_address,
                                &deserialize_payload.student_code,
                                &deserialize_payload.full_name,
                                &deserialize_payload.email,
                            )
                            .await
                            .map_err(async |e| {
                                tracing::error!("Failed to register new student: {}", e);
                                let user_repo = UserRepository::new();
                                let delete_user_db = UserRepository::delete_by_student_code(
                                    &user_repo,
                                    &deserialize_payload.student_code,
                                )
                                .await
                                .ok()
                                .expect("Failed to delete user");
                            })
                            .ok();
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
