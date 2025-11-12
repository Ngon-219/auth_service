use futures::StreamExt;
use tokio::sync::OnceCell;
use lapin::{Connection, ConnectionProperties};
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use crate::config::APP_CONFIG;

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
        let rabbit_conn = RABBITMQ_CONNECTION.get().expect("Failed to connect to rabbitMQ");
        let channel = rabbit_conn.create_channel().await.expect("created channel");

        let mut consumer = channel
            .basic_consume(
                REGISTER_NEW_USER_CHANNEL,
                "register_student",
                BasicConsumeOptions::default(),
                FieldTable::default()
            )
            .await?;

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Lỗi khi nhận message: {}", e);
                    continue; // Bỏ qua và chờ message tiếp theo
                }
            };

            match std::str::from_utf8(&delivery.data) {
                Ok(payload) => {
                    println!("\n[ĐÃ NHẬN] Payload: '{}'", payload);
                    let ack_options = BasicAckOptions::default();
                    if let Err(e) = delivery.ack(ack_options).await {
                        eprintln!("Lỗi khi gửi ACK: {}", e);
                    } else {
                        println!("[ĐÃ ACK] Xử lý xong.");
                    }
                }
                Err(_) => {
                    eprintln!("\n[LỖI] Nhận được message không phải UTF-8");

                    delivery.ack(BasicAckOptions::default()).await?;
                }
            }
        };

        Ok(())
    }
}