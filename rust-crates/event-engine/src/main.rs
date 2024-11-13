use event::Event;
use types::{init_metadata_in_redis, redis_util::redis_publish, Vehicle};
use tokio::sync::mpsc;

mod rt_gtfs_client;
mod stream_processor;
mod training_data_client;
mod event;

#[tokio::main]
async fn main() {
    init_metadata_in_redis().await;
    let (input_sender, input_receiver) = mpsc::unbounded_channel::<Event>();
    let (output_sender, mut output_receiver) = mpsc::unbounded_channel::<Event>();
    let mut processor = stream_processor::StreamProcessor::default().await;

    tokio::task::spawn(async move {
        processor.run(input_receiver, output_sender).await
    });
    rt_gtfs_client::start_gtfs_realtime_clients(input_sender);

    loop {
        let event = output_receiver.recv().await.unwrap();
        match event {
            Event::TripUpdate(..) => (),
            Event::Vehicle(vehicle) => {
                let serialized = vehicle.serialize_for_frontend();
                redis_publish("realtime-with-metadata", serialized).await;
            }
        }
    }
}
