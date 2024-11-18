/// The compiled GTFS realtime protobuf spec
pub mod transit_realtime {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

use chrono;
use lazy_static::lazy_static;
use prost::Message;
use reqwest;
use types::Vehicle;
use std::env;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use transit_realtime::{FeedMessage, VehicleDescriptor, VehiclePosition, TripUpdate as GTFSTripUpdate};

use crate::event::{Event, StopTimeUpdate, TripUpdate};

use self::transit_realtime::TripDescriptor;

lazy_static! {
    static ref TRAFIKLAB_GTFS_RT_KEY: String = env::var("TRAFIKLAB_GTFS_RT_KEY")
        .expect("required env variable not set TRAFIKLAB_GTFS_RT_KEY");
    static ref TRANSPORT_AGENCIES: Vec<String> = [
        "dt",
        "jlt",
        "klt",
        "krono",
        "orebro",
        "skane",
        "sl",
        "ul",
        "vastmanland",
        "varm",
        "xt",
        "otraf"
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
}

/// Starts the realtime GTFS API clients that poll for vehicle position updates from Samtrafiken
pub fn start_gtfs_realtime_clients(sender: UnboundedSender<Event>) {
    for agency in TRANSPORT_AGENCIES.iter() {
        let vehicle_url = format!(
            "https://opendata.samtrafiken.se/gtfs-rt-sweden/{}/VehiclePositionsSweden.pb?key={}",
            agency, *TRAFIKLAB_GTFS_RT_KEY
        );
        let trip_update_url = format!(
            "https://opendata.samtrafiken.se/gtfs-rt-sweden/{}/TripUpdatesSweden.pb?key={}",
            agency, *TRAFIKLAB_GTFS_RT_KEY
        );
        let sender_clone = sender.clone();
        tokio::task::spawn(
            async move { run_client(vehicle_url, Duration::from_secs(3), sender_clone).await },
        );
        let sender_clone = sender.clone();
        tokio::task::spawn(
            async move { run_client(trip_update_url, Duration::from_secs(15), sender_clone).await },
        );
    }
}

async fn run_client(url: String, interval: Duration, sender: UnboundedSender<Event>) {
    let client = reqwest::Client::builder().gzip(true).build().unwrap();
    let mut last_updated_time = chrono::Utc::now();
    loop {
        let resp_or_err = client
            .get(&url)
            .header("Accept", "application/octet-stream")
            .header("Accept-Encoding", "gzip")
            .header("If-Modified-Since", last_updated_time.to_rfc2822())
            .send()
            .await;
        last_updated_time = chrono::Utc::now();
        sleep(interval).await;
        match resp_or_err {
            Err(e) => println!("{:?}", e),
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    if let Ok(msg) = FeedMessage::decode(bytes.clone()) {
                        for entity in msg.entity {
                            // check for trip update
                            if let Some(GTFSTripUpdate {
                                trip: TripDescriptor { trip_id: Some(trip_id), .. },
                                stop_time_update,
                                ..
                            }) = entity.trip_update {
                                let event = Event::TripUpdate(TripUpdate {
                                    trip_id,
                                    time_updates: stop_time_update.into_iter()
                                        .filter(|update| update.stop_sequence.is_some() && update.arrival.is_some())
                                        .map(|update| {
                                            StopTimeUpdate {
                                                stop_sequence: update.stop_sequence(),
                                                arrival_unix_secs: update.arrival.unwrap().time.map(|x| x as u64),
                                                delay_secs: update.arrival.unwrap().delay,
                                            }
                                        }).collect()
                                });
                                sender.send(event).expect("internal channel broken");
                            }

                            // chech for vehicle position
                            if let Some(VehiclePosition {
                                vehicle: Some(VehicleDescriptor { id: Some(id), .. }),
                                position: Some(pos),
                                trip,
                                timestamp: Some(ts),
                                ..
                            }) = entity.vehicle
                            {
                                let event = Event::Vehicle(Vehicle {
                                    id,
                                    lat: pos.latitude,
                                    lng: pos.longitude,
                                    trip_id: trip.map(|x| x.trip_id().to_string()),
                                    timestamp: ts,
                                    metadata: None,
                                    delay: None,
                                    stop_seq: None,
                                });
                                sender.send(event).expect("internal channel broken");
                            }
                        }
                    } else {
                        println!("Protobuf Decode Error: Bytes length {}\non url {}", bytes.len(), url);
                        if let Ok(err_msg) = String::from_utf8(bytes.to_vec()) {
                            println!("message: {:?}", err_msg);
                        }
                    }
                }
            }
        }
    }
}
