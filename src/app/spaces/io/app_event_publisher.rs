use std::sync::{Arc, Mutex};
use crate::app::spaces::domain::domain_event::SpacesDomainEvent;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn spaces_app_event_publisher(app_event_bus: Arc<Mutex<EventBus>>, event_bus: Arc<Mutex<EventBus>>) {
    event_bus.lock().unwrap().subscribe_all(move |event_envelope: &EventEnvelope| {
        let event_envelope = event_envelope.clone();
        let cloned_app_event_bus = Arc::clone(&app_event_bus);

        tokio::spawn(async move {
            let Ok(event) = serde_json::from_value::<SpacesDomainEvent>(event_envelope.payload.clone()) else {
                return;
            };
            if let Some(app_event) = event.to_app_event() {
                cloned_app_event_bus.lock().unwrap().publish(&app_event.to_enveloppe());
            }
        });
    });
}

