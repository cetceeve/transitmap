use std::collections::HashMap;

use types::{Delays, Vehicle, VehicleMetadata};

use crate::event::Event;

use super::ProcessingStep;

pub struct DelayProcessor {
    delays: HashMap<String, Vec<i32>>,
    number_of_stops: HashMap<String, usize>,
}

impl DelayProcessor {
    pub fn init() -> Self {
        Self {
            delays: Default::default(),
            number_of_stops: Default::default(),
        }
    }
}

impl ProcessingStep for DelayProcessor {
    fn apply(&mut self, event: &mut Event) -> (bool, Option<(String, Vec<u8>)>) {
        match event {
            Event::Vehicle(Vehicle {
                trip_id: Some(trip_id),
                metadata: Some(VehicleMetadata{
                    stops: Some(stops),
                    ..
                }),
                ..
            }) => {
                self.number_of_stops.insert(trip_id.clone(), stops.len());
                (true, None)
            },
            Event::TripUpdate(updates) => {
                if let Some(num_stops) = self.number_of_stops.get(&updates.trip_id) {
                    let mut delays = vec![];
                    for update in updates.time_updates.iter() {
                        while update.stop_sequence as usize > delays.len() + 1 {
                            if delays.len() > 1 {
                                delays.push(*delays.last().unwrap());
                            } else {
                                delays.push(0);
                            }
                        }
                        if let Some(delay) = update.delay_secs {
                            delays.push(delay);
                        }
                    }
                    while delays.len() < *num_stops {
                        if delays.len() > 1 {
                            delays.push(*delays.last().unwrap());
                        } else {
                            delays.push(0);
                        }
                    }
                    self.delays.insert(updates.trip_id.clone(), delays.clone());
                    let real_stop_times = Delays{ trip_id: updates.trip_id.clone(), delays };
                    (false, Some((String::from("trip-updates"), serde_json::to_vec(&real_stop_times).unwrap())))
                } else {
                    (false, None)
                }
            },
            _ => (true, None),
        }
    }
}

