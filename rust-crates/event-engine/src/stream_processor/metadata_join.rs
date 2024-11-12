use types::redis_util::redis_get;
use async_trait::async_trait;

use crate::event::Event;

use super::ProcessingStep;

pub struct MetadataJoiner {}

impl MetadataJoiner {
    pub fn init() -> Self {
        Self {}
    }
}

#[async_trait]
impl ProcessingStep for MetadataJoiner {
    async fn apply(&self, event: &mut Event) -> bool {
        if let Event::Vehicle(vehicle) = event {
            if let Some(ref trip_id) = vehicle.trip_id {
                let key = String::from("trip_meta:") + trip_id;
                vehicle.metadata = redis_get(&key).await.ok();
                true
            } else {
                false
            }
        } else {
            true
        }
    }
}
