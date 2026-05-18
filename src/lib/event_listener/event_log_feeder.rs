use std::sync::Arc;
use sqlx::PgPool;
use std::sync::Mutex;
use crate::lib::event_envelope::EventEnvelope;
use crate::lib::persistance::event_log_repository::{EventLogRepository, IEventLogRepository};
use crate::lib::services::event_bus::event_bus::{EventBus, EventTag};

pub fn init(bus: Arc<Mutex<EventBus>>, pool: PgPool) {
    let repo = Arc::new(EventLogRepository::new(pool));
    let mut bus  = bus.lock().unwrap();

    bus.subscribe_all(move |event: &EventEnvelope | {
        let repo  = Arc::clone(&repo);
        let event = event.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.save(&event).await {
                tracing::error!("event_log: failed to persist '{}': {e}", event.event_type);
            }
        });
    });
}