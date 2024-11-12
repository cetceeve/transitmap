use serde::{Deserialize, Serialize};
use types::Vehicle;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Vehicle(Vehicle),
    TripUpdate(TripUpdate),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TripUpdate {
    pub trip_id: String,
    pub time_updates: Vec<StopTimeUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StopTimeUpdate {
    pub stop_sequence: u32,
    pub arrival_unix_secs: Option<u64>,
    pub delay_secs: Option<i32>,
}

