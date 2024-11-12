use std::collections::HashMap;
use std::sync::OnceLock;

use axum::{extract::Path, Json, http::StatusCode};
use tokio::sync::Mutex;
use types::{get_trip_metadata_async, Delays, VehicleMetadata};
use types::redis_util::subscribe;

static REAL_STOP_TIMES: OnceLock<Mutex<HashMap<String, Vec<i32>>>> = OnceLock::new();

pub async fn init_delay_listener() {
    REAL_STOP_TIMES.get_or_init(|| { Mutex::new(Default::default()) });
    let mut receiver = subscribe("trip-updates").await;
    tokio::spawn(async move {
        loop {
            let item = receiver.recv().await.expect("broken channel");
            let real_stop_times: Delays = serde_json::from_str(&item).expect("bad message (trip update)");
            REAL_STOP_TIMES.get().unwrap().lock().await.insert(real_stop_times.trip_id, real_stop_times.delays);
        }
    });
}

pub async fn metadata_handler(Path(trip_id): Path<String>) -> Result<Json<VehicleMetadata>, StatusCode> {
    if let Some(mut metadata) = get_trip_metadata_async(&trip_id).await {
        if let Some(table) = REAL_STOP_TIMES.get() {
            metadata.delays = table.lock().await.get(&trip_id).map(|x| x.to_owned());
        }
        Ok(Json(metadata))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
