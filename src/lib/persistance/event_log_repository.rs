use sqlx::PgPool;
use time::OffsetDateTime;
use crate::app::shared_kernel::domain_event::DomainEvent;

#[derive(Debug, sqlx::FromRow)]
pub struct StoredEvent {
    pub id:                 String,
    pub source:             String,
    pub event_type:         String,
    pub spec_version:       String,
    pub time:               OffsetDateTime,
    pub data_schema:        String,
    pub data_content_type:  Option<String>,
    pub subject:            Option<String>,
    pub data:               Option<String>,
}

pub trait IEventLogRepository: Send + Sync {
    async fn find_by_subject(&self, subject: &str) -> Result<Vec<StoredEvent>, sqlx::Error>;
    async fn save(&self, event: &StoredEvent) -> Result<(), sqlx::Error>;

    async fn save_event<E: DomainEvent>(
        &self,
        id: String,
        user_id: Option<String>,
        subject: Option<String>,
        event: &E,
    ) -> Result<(), sqlx::Error> {
        let stored = StoredEvent {
            id,
            source:            user_id.unwrap_or_else(|| "/".to_string()),
            event_type:        E::event_type().to_string(),
            spec_version:      E::version().to_string(),
            time:              OffsetDateTime::now_utc(),
            data_schema:       E::schema().to_string(),
            data_content_type: Some("application/json".to_string()),
            subject,
            data:              event.serialize_data(),
        };
        self.save(&stored).await
    }
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
    async fn find_by_subject(&self, subject: &str) -> Result<Vec<StoredEvent>, sqlx::Error> {
        sqlx::query_file_as!(
            StoredEvent,
            "src/lib/persistance/sql/find_by_subject.sql",
            subject
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn save(&self, event: &StoredEvent) -> Result<(), sqlx::Error> {
        sqlx::query_file!(
            "src/lib/persistance/sql/insert_event.sql",
            event.id,
            event.source,
            event.event_type,
            event.spec_version,
            event.time,
            event.data_schema,
            event.data_content_type,
            event.subject,
            event.data,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}