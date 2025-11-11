use crate::config::APP_CONFIG;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use socketioxide_emitter::{Driver, IoEmitter};
use tokio::sync::OnceCell;
struct RedisConnection(MultiplexedConnection);
impl Driver for RedisConnection {
    type Error = redis::RedisError;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        self.0
            .clone()
            .publish::<_, _, redis::Value>(channel, data)
            .await?;
        Ok(())
    }
}

pub static REDIS_EMITTER: OnceCell<RedisConnection> = OnceCell::const_new();

pub async fn get_redis_emitter_conn() -> &'static RedisConnection {
    REDIS_EMITTER
        .get_or_init(|| async {
            let client = redis::Client::open(APP_CONFIG.redis_url.as_str())
                .expect("Failed to create Redis client");
            let conn = client
                .get_multiplexed_tokio_connection()
                .await
                .expect("Failed to connect to Redis");
            RedisConnection(conn)
        })
        .await
}

pub struct RedisEmitter;

impl RedisEmitter {
    pub async fn emit_to_rooom(room: &str, msg: &str) {
        let redis_emitter = get_redis_emitter_conn().await;
        IoEmitter::new()
            .to(room.to_string())
            .emit("event", msg, redis_emitter)
            .await
            .expect("Failed to emit redis");
    }

    pub async fn emit_to_all(msg: &str) {
        let redis_emitter = get_redis_emitter_conn().await;
        IoEmitter::new()
            .emit("event", msg, redis_emitter)
            .await
            .expect("Failed to emit redis");
    }
}
