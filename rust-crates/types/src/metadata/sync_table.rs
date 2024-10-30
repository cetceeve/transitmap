use std::{collections::HashMap, sync::{Mutex, OnceLock, RwLock}, time::{Duration, SystemTime}};

use super::{VehicleMetadata, download_metadata_table_blocking};

static mut TABLE: OnceLock<RwLock<HashMap<String, VehicleMetadata>>> = OnceLock::new();
static mut LAST_UPDATED: OnceLock<Mutex<SystemTime>> = OnceLock::new();

fn ensure_table() {
    unsafe {
        if TABLE.get().is_none() {
            LAST_UPDATED.get_or_init(|| {Mutex::new(SystemTime::now())});
            TABLE.get_or_init(|| { RwLock::new(Default::default()) });
            let mut lock = TABLE.get().unwrap().write().unwrap();
            *lock = download_metadata_table_blocking().unwrap();
        } else {
            let last_updated = LAST_UPDATED.get_or_init(|| {Mutex::new(SystemTime::now())});
            let mut ts = last_updated.lock().unwrap();
            if ts.elapsed().unwrap() > Duration::from_secs(24*60*60) {
                let table = TABLE.get_or_init(|| { RwLock::new(Default::default()) });
                let data = download_metadata_table_blocking().unwrap();
                *table.write().unwrap() = data;
                *ts = SystemTime::now();
            }
        }
    }
}

pub fn init_blocking_metadata_table() {
    ensure_table();
}

pub fn get_trip_metadata_blocking(trip_id: &str) -> Option<VehicleMetadata> {
    ensure_table();
    unsafe {
        if let Some(table) = TABLE.get() {
            table.read().unwrap().get(trip_id).cloned()
        } else {
            None
        }
    }
}
