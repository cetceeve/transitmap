use types::get_trip_metadata_blocking;

use crate::event::Event;

use super::ProcessingStep;

pub struct MetadataJoiner {}

impl MetadataJoiner {
    pub fn init() -> Self {
        Self {}
    }
}

impl ProcessingStep for MetadataJoiner {
    fn apply(&mut self, event: &mut Event) -> (bool, Option<(String, Vec<u8>)>) {
        if let Event::Vehicle(vehicle) = event {
            if let Some(ref trip_id) = vehicle.trip_id {
                vehicle.metadata = get_trip_metadata_blocking(trip_id);
                (true, None)
            } else {
                (false, None)
            }
        } else {
            (true, None)
        }
    }
}
