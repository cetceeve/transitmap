use std::time::Duration;

use crate::redis_util::redis_set;

use super::download_metadata_table_async;

async fn update_metadata_in_redis() {
    let data = download_metadata_table_async().await.unwrap();
    for (trip_id, value) in data.into_iter() {
        let key = String::from("trip_meta:") + &trip_id;
        let _ = redis_set(&key, value, Some(60 * 60 * 24 * 5)).await; // TTL: 5 days
    }
    println!("updated trip metadata in redis");
}

/// Must be called exactly once and only by event engine.
pub async fn init_metadata_in_redis() {
    tokio::spawn(async move {
        loop {
            update_metadata_in_redis().await;
            tokio::time::sleep(Duration::from_secs(60 * 60 * 24)).await;
        }
    });
}
