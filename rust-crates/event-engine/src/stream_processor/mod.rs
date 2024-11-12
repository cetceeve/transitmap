use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use redis::AsyncCommands;

mod metadata_join;
mod delay_processor;

use crate::event::Event;

use self::delay_processor::DelayProcessor;
use self::metadata_join::MetadataJoiner;

pub trait ProcessingStep: Send {
    /// May mutate the vehicle, or remove it from the stream, by returning `(false, _)`.
    /// May return an arbitrary binary message to publish to redis on a topic by returning `(_, Some(topic_name, msg))`
    fn apply(&mut self, vehicle: &mut Event) -> (bool, Option<(String, Vec<u8>)>);
}

/// Processes a steam of Vehicle events.
pub struct StreamProcessor {
    /// registered processing steps
    processing_steps: Vec<Box<dyn ProcessingStep>>,
}

impl StreamProcessor {
    pub fn init() -> Self {
        Self {
            processing_steps: vec![],
        }
    }

    pub fn register_step(&mut self, step: Box<dyn ProcessingStep>) {
        self.processing_steps.push(step);
    }

    pub async fn default() -> Self {
        let mut processor = Self::init();
        processor.register_step(Box::new(MetadataJoiner::init()));
        processor.register_step(Box::new(DelayProcessor::init()));
        processor
    }

    pub async fn run(
        &mut self,
        mut receiver: UnboundedReceiver<Event>,
        sender: UnboundedSender<Event>,
    ) {
        let redis_client = redis::Client::open("redis://sparkling-redis/").unwrap();
        let mut redis_conn = redis_client.get_tokio_connection().await.unwrap();
        'EVENT_LOOP: loop {
            let mut vehicle = receiver.recv().await.expect("broken internal channel");

            // precessing steps
            for step in &mut self.processing_steps {
                let (keep_event, message) = step.apply(&mut vehicle);
                if let Some((topic_name, data)) = message {
                    redis_conn.publish::<_,_,()>(topic_name, data).await.unwrap();
                }
                if !keep_event {
                    continue 'EVENT_LOOP
                }
            }

            sender.send(vehicle).expect("broken internal channel");
        }
    }
}
