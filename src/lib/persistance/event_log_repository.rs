use sqlx::PgPool;
use time::OffsetDateTime;

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

pub struct EventLogRepository {
    pool: PgPool,
}

impl EventLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_subject(&self, subject: &str) -> Result<Vec<StoredEvent>, sqlx::Error> {
        sqlx::query_file_as!(
            StoredEvent,
            "src/lib/persistance/sql/find_by_subject.sql",
            subject
        )
        .fetch_all(&self.pool)
        .await
    }
}