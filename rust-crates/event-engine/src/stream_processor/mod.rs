use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use async_trait::async_trait;

mod metadata_join;
mod delay_processor;

use crate::event::Event;

use self::delay_processor::DelayProcessor;
use self::metadata_join::MetadataJoiner;

#[async_trait]
pub trait ProcessingStep: Send + Sync {
    /// May mutate the event, or remove it from the stream, by returning `false`.
    async fn apply(&self, event: &mut Event) -> bool;
}

/// Processes a steam of Vehicle events.
pub struct StreamProcessor {
    /// registered processing steps
    processing_steps: Arc<Vec<Box<dyn ProcessingStep>>>,
}

impl StreamProcessor {
    pub fn init(steps: Vec<Box<dyn ProcessingStep>>) -> Self {
        Self {
            processing_steps: Arc::new(steps),
        }
    }

    pub async fn default() -> Self {
        let mut steps: Vec<Box<dyn ProcessingStep>> = vec![];
        steps.push(Box::new(MetadataJoiner::init()));
        steps.push(Box::new(DelayProcessor::init()));
        Self::init(steps)
    }

    pub async fn run(
        &mut self,
        mut receiver: UnboundedReceiver<Event>,
        sender: UnboundedSender<Event>,
    ) {
        loop {
            let mut event = receiver.recv().await.expect("broken internal channel");

            let sender_clone = sender.clone();
            let steps = self.processing_steps.clone();
            tokio::spawn(async move {
                for step in steps.iter() {
                    let keep_event = step.apply(&mut event).await;
                    if !keep_event {
                        break
                    }
                }
                sender_clone.send(event).expect("broken internal channel");
            });

        }
    }
}
