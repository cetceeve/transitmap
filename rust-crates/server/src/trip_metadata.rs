use axum::{extract::Path, Json, http::StatusCode};
use types::VehicleMetadata;
use types::redis_util::redis_get;

pub async fn metadata_handler(Path(trip_id): Path<String>) -> Result<Json<VehicleMetadata>, StatusCode> {

    let meta_key = String::from("trip_meta:") + &trip_id;
    let delay_key = String::from("delays:") + &trip_id;
    
    if let Some(mut metadata) = redis_get::<VehicleMetadata>(&meta_key).await.ok() {
        let delays = redis_get::<Vec<i32>>(&delay_key).await.ok();
        metadata.delays = delays;
        Ok(Json(metadata))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
