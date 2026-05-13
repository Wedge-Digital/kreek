use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use time::OffsetDateTime;
use tower_sessions::{
    session::{Id, Record},
    session_store, SessionStore,
};

/// Session store backed by DashMap — concurrent reads and per-shard writes,
/// no global mutex. Drop-in replacement for MemoryStore in dev/staging.
#[derive(Clone, Debug, Default)]
pub struct DashMapStore(Arc<DashMap<Id, Record>>);

impl DashMapStore {
    pub fn new() -> Self {
        Self(Arc::new(DashMap::new()))
    }
}

#[async_trait]
impl SessionStore for DashMapStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        tracing::debug!(session_id = %record.id, "store: create");
        while self.0.contains_key(&record.id) {
            record.id = Id::default();
        }
        self.0.insert(record.id, record.clone());
        tracing::debug!(session_id = %record.id, "store: create done");
        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        tracing::debug!(session_id = %record.id, "store: save");
        self.0.insert(record.id, record.clone());
        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        tracing::debug!(%session_id, "store: load");
        let result = self
            .0
            .get(session_id)
            .filter(|r| r.expiry_date > OffsetDateTime::now_utc())
            .map(|r| r.clone());
        tracing::debug!(%session_id, found = result.is_some(), "store: load done");
        Ok(result)
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        tracing::debug!(%session_id, "store: delete");
        self.0.remove(session_id);
        Ok(())
    }
}
