use std::collections::HashMap;
use async_trait::async_trait;

use tokio::sync::Mutex;
use types::{redis_util::{redis_set, redis_get}, Vehicle, VehicleMetadata};

use crate::event::Event;

use super::ProcessingStep;

pub struct DelayProcessor {
    number_of_stops: Mutex<HashMap<String, usize>>,
}

impl DelayProcessor {
    pub fn init() -> Self {
        Self {
            number_of_stops: Default::default(),
        }
    }
}

#[async_trait]
impl ProcessingStep for DelayProcessor {
    async fn apply(&self, event: &mut Event) -> bool {
        match event {
            Event::Vehicle(Vehicle {
                trip_id: Some(trip_id),
                metadata: Some(VehicleMetadata{
                    stops: Some(stops),
                    ..
                }),
                ..
            }) => {
                self.number_of_stops.lock().await.insert(trip_id.clone(), stops.len());
                true
            },
            Event::TripUpdate(updates) => {
                let num_stops = self.number_of_stops.lock().await.get(&updates.trip_id).map(|x| x.to_owned());
                if let Some(num_stops) = num_stops {
                    let key = String::from("delays:") + &updates.trip_id;
                    let mut delays = redis_get::<Vec<i32>>(&key).await.unwrap_or_else(|_| Vec::<i32>::with_capacity(32));
                    for update in updates.time_updates.iter() {
                        while update.stop_sequence as usize > delays.len() + 1 {
                            if delays.len() > 1 {
                                delays.push(*delays.last().unwrap());
                            } else {
                                delays.push(0);
                            }
                        }
                        if let Some(delay) = update.delay_secs {
                            if delays.len() >= update.stop_sequence as usize {
                                delays[update.stop_sequence as usize - 1] = delay;
                            } else {
                                delays.push(delay);
                            }
                        }
                    }
                    while delays.len() < num_stops {
                        if delays.len() > 1 {
                            delays.push(*delays.last().unwrap());
                        } else {
                            delays.push(0);
                        }
                    }
                    let _ = redis_set(&key, delays, Some(1000)).await.unwrap();
                    false
                } else {
                    false
                }
            },
            _ => true
        }
    }
}

