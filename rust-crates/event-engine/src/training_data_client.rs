use anyhow::Result;
use std::io::Read;
use std::time::Duration;

use flate2::read::GzDecoder;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

use crate::Vehicle;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    bucket: String,
    name: String,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CsvVehicle {
    pub id: String,
    pub lng: f32,
    pub lat: f32,
    pub timestamp: u64,
    pub trip_id: Option<String>,
    pub route_type: Option<u64>,
    pub agency_name: Option<String>,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub trip_headsign: Option<String>,
    pub shape_id: Option<u64>,
    pub direction_id: Option<u32>,
}

pub struct TrainingDataClient {
    file_index: usize,
    files: Vec<FileInfo>,
}

impl TrainingDataClient {
    pub async fn init(file_index: usize) -> Self {
        Self {
            file_index,
            files: get_file_list().await.unwrap(),
        }
    }

    /// Setup client to feed all training data
    pub async fn default() -> Self {
        Self::init(0).await
    }

    /// Continously downloads training data from google storage and pushes into sender
    /// Currently configured to only return metros
    pub async fn run(&mut self, sender: Sender<Vehicle>) {
        let client = Client::new(ClientConfig::default().anonymous());

        self.file_index = 30; // start at 30, because 0 is at night

        let mut event_counter: u64 = 0;
        while self.file_index < self.files.len() {
            println!("Processing file {:?}/{:?}", self.file_index+1, self.files.len());
            let csv_file = get_file(&client, self.files[self.file_index].clone()).await;
            if let Ok(rows) = csv_file {
                for record in rows {
                    // only send 1000 events per second
                    event_counter+=1;
                    if event_counter == 1000 {
                        sleep(Duration::from_secs(1)).await;
                        event_counter = 0;
                    }
                    // Send with backpressure!
                    sender.send(Vehicle {
                        id: record.id,
                        lng: record.lng,
                        lat: record.lat,
                        timestamp: record.timestamp,
                        trip_id: record.trip_id,
                        metadata: None,
                        delay: None,
                        stop_seq: None,
                    }).await.expect("Internal channel broken.");
                }
                self.file_index = self.file_index + 1;
            } else {
                println!("{:#?}", self.files[self.file_index]);
                panic!("Training file download and desialisation failed.");
            }
        }
    }
}

async fn get_file_list() -> Result<Vec<FileInfo>> {
    let client = Client::new(ClientConfig::default().anonymous());

    let (mut files, mut page_token) = get_file_list_page(&client, None).await?;
    while page_token != None {
        let (mut next_files, next_page_token) = get_file_list_page(&client, page_token).await?;
        files.append(&mut next_files);
        page_token = next_page_token;
    }

    println!("{:#?} training files ready to process.", files.len());
    Ok(files)
}

async fn get_file_list_page(
    client: &Client,
    page_token: Option<String>,
) -> Result<(Vec<FileInfo>, Option<String>)> {
    let res = client
        .list_objects(&ListObjectsRequest {
            bucket: "rt-archive".to_string(),
            page_token,
            ..Default::default()
        })
        .await?;
       

    let list: Vec<FileInfo> = if let Some(items) = res.items {
        items
            .into_iter()
            .map(|x| FileInfo {
                bucket: x.bucket,
                name: x.name,
            })
            .collect()
    } else {
        Vec::default()
    };

    Ok((list, res.next_page_token))
}

/// Downloads and desializes single file.
async fn get_file(client: &Client, file_info: FileInfo) -> Result<Vec<CsvVehicle>> {
    let data = client
        .download_object(
            &GetObjectRequest {
                bucket: file_info.bucket,
                object: file_info.name,
                ..Default::default()
            },
            &Range::default(),
        )
        .await?;

    // Explisitly decompress the gziped body because there is no content-encoding: gzip header on the response
    let mut buffer: Vec<u8> = Vec::default();
    let mut decoder = GzDecoder::new(data.as_slice());
    // Keep it as bytes since the csv reader deserializes from bytes anyway
    decoder.read_to_end(&mut buffer)?;

    // Desialize directly into vector, fails if one deserialization fails
    let res: Result<Vec<CsvVehicle>, csv::Error> = csv::Reader::from_reader(buffer.as_slice())
        .deserialize()
        .collect();

    Ok(res?)
}
