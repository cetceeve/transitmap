use std::{collections::HashMap, sync::OnceLock};

use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{broadcast::{Sender, Receiver, self}, Mutex};
use redis::{self, AsyncCommands, FromRedisValue, PushKind};

lazy_static!{
    static ref REDIS_BROADCASTS: Mutex<HashMap<String, Sender<String>>> = Default::default();
    static ref REDIS_CLIENT: redis::Client = redis::Client::open("redis://sparkling-redis/?protocol=resp3").unwrap();
    static ref REDIS_CONN: OnceLock<redis::aio::ConnectionManager> = OnceLock::new();
}

async fn connection() -> redis::aio::ConnectionManager {
    let option = REDIS_CONN.get();
    if let Some(conn) = option {
        conn.clone()
    } else {
        let conn = redis::aio::ConnectionManager::new(redis::Client::open("redis://sparkling-redis/?protocol=resp3").unwrap()).await.unwrap();
        REDIS_CONN.get_or_init(|| conn).clone()
    }
}

// We serialize values ourselves, since the redis crate messes with some types (specifically Vectors).
pub async fn redis_set<V: Serialize + Send + Sync>(key: &str, value: V, ttl_secs: Option<u64>) -> anyhow::Result<()> {
    let mut conn = connection().await;
    let data = bincode::serialize(&value)?;
    if let Some(ttl) = ttl_secs {
        conn.set_ex(key, data, ttl).await?;
    } else {
        conn.set(key, data).await?;
    }
    Ok(())
}

pub async fn redis_get<V: DeserializeOwned>(key: &str) -> anyhow::Result<V> {
    let data: Vec<u8> = connection().await.get(key).await?;
    let value: V = bincode::deserialize(&data)?;
    Ok(value)
}

pub async fn subscribe(topic: &str) -> Receiver<String> {
    let mut locked_broadcasts = REDIS_BROADCASTS.lock().await;
    if let Some(broadcast) = locked_broadcasts.get(topic) {
        // we have an active subscriber for this topic
        broadcast.subscribe()
    } else {
        // create a new subscriber for this topic
        let (sender, receiver) = broadcast::channel::<String>(5000);
        locked_broadcasts.insert(topic.to_string(), sender.clone());
        let topic = topic.to_string();
        tokio::task::spawn(async move { run_subscriber(&topic, sender).await });
        receiver
    }
}

async fn run_subscriber(topic: &str, sender: Sender<String>) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let config = redis::AsyncConnectionConfig::new().set_push_sender(tx);
    let mut conn = REDIS_CLIENT.get_multiplexed_async_connection_with_config(&config).await.unwrap();
    conn.subscribe(topic).await.unwrap();

    while let Some(msg) = rx.recv().await {
        match msg.kind {
            PushKind::Message => {
                for item in msg.data {
                    if let Ok(data) = String::from_redis_value(&item) {
                        let _ = sender.send(data);
                    }
                }
            },
            _ => (),
        }
    }
}
