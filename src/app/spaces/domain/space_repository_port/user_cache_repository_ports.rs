
use crate::app::shared_kernel::common_types::{CoachId, SpaceId};
use async_trait::async_trait;
use std::fmt;
use crate::app::spaces::domain::user::User;

pub struct SpaceUserCache {
    pub id:   String,
    pub name: String,
    pub logo: String,
    pub email: String,
}

#[derive(Debug)]
pub enum SpaceUserCacheRepositoryError {
    UsernameNameAlreadyPresentInCache,
    Database(String),
}

impl fmt::Display for SpaceUserCacheRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache => { f.write_str("Username already present in cache")}
            SpaceUserCacheRepositoryError::Database(_) => { f.write_str("Database error") }
        }
    }
}

impl std::error::Error for SpaceUserCacheRepositoryError {}

#[async_trait]
pub trait ISpaceUserCacheRepository: Send + Sync {
    async fn save(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError>;
    async fn find_by_id(&self, id: &CoachId) -> Result<Option<User>, SpaceUserCacheRepositoryError>;
    async fn find_all(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError>;
}