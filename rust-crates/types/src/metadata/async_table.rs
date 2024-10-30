use std::{collections::HashMap, sync::OnceLock, time::{Duration, SystemTime}};
use tokio::sync::{RwLock, Mutex};

use super::{VehicleMetadata, download_metadata_table_async};

static mut TABLE: OnceLock<RwLock<HashMap<String, VehicleMetadata>>> = OnceLock::new();
static mut LAST_UPDATED: OnceLock<Mutex<SystemTime>> = OnceLock::new();

async fn ensure_table() {
    unsafe {
        if TABLE.get().is_none() {
            LAST_UPDATED.get_or_init(|| {Mutex::new(SystemTime::now())});
            TABLE.get_or_init(|| { RwLock::new(Default::default()) });
            let mut lock = TABLE.get().unwrap().write().await;
            *lock = download_metadata_table_async().await.unwrap();
        }
    }
}

async fn update_table() {
    unsafe {
        let last_updated = LAST_UPDATED.get_or_init(|| {Mutex::new(SystemTime::now())});
        let mut ts = last_updated.lock().await;
        if ts.elapsed().unwrap() > Duration::from_secs(24*60*60) {
            let table = TABLE.get_or_init(|| { RwLock::new(Default::default()) });
            let data = download_metadata_table_async().await.unwrap();
            *table.write().await = data;
            *ts = SystemTime::now();
        }
    }
}

pub async fn init_async_metadata_table() {
    ensure_table().await;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(500)).await;
            update_table().await;
        }
    });
}

pub async fn get_trip_metadata_async(trip_id: &str) -> Option<VehicleMetadata> {
    ensure_table().await;
    unsafe {
        if let Some(table) = TABLE.get() {
            table.read().await.get(trip_id).cloned()
        } else {
            None
        }
    }
}
