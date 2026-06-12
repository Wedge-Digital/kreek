use crate::common::persistance::event_log_repository::{EventLogRepository, IEventLogRepository};
use crate::common::services::event_bus::event_bus::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

pub fn init(bus: &EventBus, pool: PgPool) {
    let repo = Arc::new(EventLogRepository::new(pool));
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = repo.save(&event).await {
                        tracing::error!("event_log: failed to persist '{}': {e}", event.event_type);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("event_log: lagged by {n} messages");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}
