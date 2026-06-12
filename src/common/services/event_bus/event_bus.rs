use crate::common::event_envelope::EventEnvelope;
use tokio::sync::broadcast;

pub type EventBus = broadcast::Sender<EventEnvelope>;

pub fn new_bus() -> EventBus {
    broadcast::channel(256).0
}
