use anyhow::Result;
use csv;
use flate2::read::GzDecoder;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, Map};
use std::{collections::HashMap, io::Read};

mod sync_table;
mod async_table;

pub use sync_table::{get_trip_metadata_blocking, init_blocking_metadata_table};
pub use async_table::{get_trip_metadata_async, init_async_metadata_table};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VehicleMetadata {
    pub trip_id: String,
    pub route_type: Option<u64>,
    pub agency_name: Option<String>,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub trip_headsign: Option<String>,
    pub shape_id: Option<u64>,
    pub route_id: Option<String>, // TODO: make some of these not optional
    pub direction_id: Option<u8>,
    pub stops: Option<Vec<Stop>>, // sorted by stop_sequence
    pub delays: Option<Vec<i32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stop {
    pub stop_sequence: u16,
    pub stop_id: String,
    pub stop_name: String,
    pub arrival_time: String,
    pub departure_time: String,
    pub stop_lat: f32,
    pub stop_lon: f32,
    pub shape_dist_traveled: f32,
}

/// Used for serialization to send to the frontend
impl From<Stop> for Value {
    fn from(value: Stop) -> Self {
        let mut map = Map::new();
        map.insert("stop_sequence".to_string(), Value::from(value.stop_sequence));
        map.insert("stop_name".to_string(), Value::from(value.stop_name));
        map.insert("stop_lon".to_string(), Value::from(value.stop_lon));
        map.insert("stop_lat".to_string(), Value::from(value.stop_lat));
        map.insert("arrival_time".to_string(), Value::from(value.arrival_time));
        Value::Object(map)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct IntermediateVehicleMetadata {
    pub trip_id: String,
    pub route_id: Option<String>,
    pub route_type: Option<u64>,
    pub agency_name: Option<String>,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub trip_headsign: Option<String>,
    pub shape_id: Option<u64>,
    pub direction_id: Option<u8>,
    pub stop_id: Option<String>,
    pub stop_name: Option<String>,
    pub stop_sequence: Option<String>,
    pub stop_lat: Option<String>,
    pub stop_lon: Option<String>,
    pub arrival_time: Option<String>,
    pub departure_time: Option<String>,
    pub shape_dist_traveled: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeserializedVehicleMetadata {
    pub trip_id: String,
    pub route_id: Option<String>,
    pub route_type: Option<u64>,
    pub agency_name: Option<String>,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub trip_headsign: Option<String>,
    pub shape_id: Option<u64>,
    pub direction_id: Option<u32>,
    pub stop_id: Option<Vec<u64>>,
    pub stop_name: Option<Vec<String>>,
    pub stop_sequence: Option<Vec<u8>>,
    pub stop_lat: Option<Vec<f64>>,
    pub stop_lon: Option<Vec<f64>>,
    pub arrival_time: Option<Vec<String>>,
    pub departure_time: Option<Vec<String>>,
    pub shape_dist_traveled: Option<Vec<f64>>,
}

pub fn download_metadata_table_blocking() -> Result<HashMap<String, VehicleMetadata>> {
    let body =
        reqwest::blocking::get("https://storage.googleapis.com/gtfs_static/sweden_aggregated_metadata.csv.gz")?
            .bytes()?
            .to_vec();
    decode_table(body)
}

pub async fn download_metadata_table_async() -> Result<HashMap<String, VehicleMetadata>> {
    let body =
        reqwest::get("https://storage.googleapis.com/gtfs_static/sweden_aggregated_metadata.csv.gz")
            .await?
            .bytes()
            .await?
            .to_vec();
    decode_table(body)
}

fn decode_table(body: Vec<u8>) -> Result<HashMap<String, VehicleMetadata>> {
    // Explisitly decompress the gziped body because there is no content-encoding: gzip header on the response
    let mut buffer: Vec<u8> = Vec::default();
    let mut decoder = GzDecoder::new(body.as_slice());
    // Keep it as bytes since the csv reader deserializes from bytes anyway
    decoder.read_to_end(&mut buffer)?;

    let mut table = HashMap::<String, VehicleMetadata>::default();
    for result in csv::Reader::from_reader(buffer.as_slice()).deserialize() {
        let raw_item: IntermediateVehicleMetadata = result?;
        let item = deserialize_stops(raw_item);
        table.insert(item.trip_id.clone(), item);
    }
    println!("Downloaded metadata table");
    Ok(table)
}

fn deserialize_stops(raw_item: IntermediateVehicleMetadata) -> VehicleMetadata {
    let stop_ids: Vec<String> = if let Some(series) = raw_item.stop_id {
        series.split("|").map(|x| x.to_owned()).collect()
    } else { Vec::default() };

    let stop_names: Vec<String> = if let Some(series) = raw_item.stop_name {
        series.split("|").map(|x| x.to_owned()).collect()
    } else { Vec::default() };
    
    let stop_lats: Vec<f32> = if let Some(series) = raw_item.stop_lat {
        series.split("|").map(|x| x.parse::<f32>().unwrap()).collect()
    } else { Vec::default() };
    
    let stop_lons: Vec<f32> = if let Some(series) = raw_item.stop_lon {
        series.split("|").map(|x| x.parse::<f32>().unwrap()).collect()
    } else { Vec::default() }; 
    
    let stop_sequences: Vec<u16> = if let Some(series) = raw_item.stop_sequence {
        series.split("|").map(|x| x.parse::<u16>().unwrap()).collect()
    } else { Vec::default() };
    
    let arrival_times: Vec<String> = if let Some(series) = raw_item.arrival_time {
        series.split("|").map(|x| x.to_owned()).collect()
    } else { Vec::default() };
    
    let departure_times: Vec<String> = if let Some(series) = raw_item.departure_time {
        series.split("|").map(|x| x.to_owned()).collect()
    } else { Vec::default() };
    
    let shape_dist_traveleds: Vec<f32> = if let Some(series) = raw_item.shape_dist_traveled {
        series.split("|").map(|x| x.parse::<f32>().unwrap()).collect()
    } else { Vec::default() };

    let mut stops: Vec<Stop> = Vec::default();
    let mut i = 0;
    while i < stop_ids.len() {
       stops.insert(i, Stop {
        stop_id: stop_ids[i].to_owned(),
        stop_name: stop_names[i].to_owned(),
        stop_lat: stop_lats[i],
        stop_lon: stop_lons[i],
        stop_sequence: stop_sequences[i],
        arrival_time: arrival_times[i].to_owned(),
        departure_time: departure_times[i].to_owned(),
        shape_dist_traveled: shape_dist_traveleds[i],
       });
       i = i + 1;
    };

    VehicleMetadata {
        trip_id: raw_item.trip_id,
        shape_id: raw_item.shape_id,
        direction_id: raw_item.direction_id,
        route_id: raw_item.route_id,
        route_short_name: raw_item.route_short_name,
        route_long_name: raw_item.route_long_name,
        route_type: raw_item.route_type,
        agency_name: raw_item.agency_name,
        trip_headsign: raw_item.trip_headsign,
        stops: if stop_ids.len() > 0 { Some(stops) } else { None },
        delays: None,
    }
}
