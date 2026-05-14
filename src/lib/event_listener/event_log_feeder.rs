use std::sync::Arc;
use sqlx::PgPool;
use crate::lib::persistance::event_log_repository::{EventLogRepository, IEventLogRepository};
use crate::lib::services::event_bus::event_bus::EventBus;

pub fn init(bus: &mut EventBus, pool: PgPool) {
    let repo = Arc::new(EventLogRepository::new(pool));

    bus.subscribe_all(move |event| {
        let repo  = Arc::clone(&repo);
        let event = event.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.save(&event).await {
                tracing::error!("event_log: failed to persist '{}': {e}", event.event_type);
            }
        });
    });
}