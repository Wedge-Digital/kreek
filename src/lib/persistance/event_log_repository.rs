use sqlx::PgPool;
use crate::lib::domain_event::DomainEventEnvelope;

#[derive(Debug, sqlx::FromRow)]
pub struct StoredEvent {
    pub global_position: i64,
    pub event_id:        String,
    pub emitter:         String,
    pub event_type:      String,
    pub tags:            serde_json::Value,
    pub payload:         serde_json::Value,
    pub occurred_at:     time::OffsetDateTime,
    pub recorded_at:     time::OffsetDateTime,
}

pub trait IEventLogRepository: Send + Sync {
    fn find_by_tag(&self, tag_filter: serde_json::Value) -> impl std::future::Future<Output = Result<Vec<StoredEvent>, sqlx::Error>> + Send;
    fn save(&self, event: &DomainEventEnvelope) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

pub struct EventLogRepository {
    pool: PgPool,
}

impl EventLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl IEventLogRepository for EventLogRepository {
    async fn find_by_tag(&self, tag_filter: serde_json::Value) -> Result<Vec<StoredEvent>, sqlx::Error> {
        sqlx::query_file_as!(
            StoredEvent,
            "src/lib/persistance/sql/find_by_tag.sql",
            tag_filter,
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn save(&self, event: &DomainEventEnvelope) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO event_log (event_id, emitter, event_type, tags, payload, occurred_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&event.event_id)
        .bind(&event.emitter)
        .bind(&event.event_type)
        .bind(&event.tags)
        .bind(&event.payload)
        .bind(event.occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}