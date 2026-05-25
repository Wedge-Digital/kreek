
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
    UserNotFoundInCache,
    Database(String),
}

impl fmt::Display for SpaceUserCacheRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpaceUserCacheRepositoryError::UsernameNameAlreadyPresentInCache => f.write_str("Username already present in cache"),
            SpaceUserCacheRepositoryError::UserNotFoundInCache               => f.write_str("User not found in cache"),
            SpaceUserCacheRepositoryError::Database(e)                       => write!(f, "Database error: {e}"),
        }
    }
}

impl std::error::Error for SpaceUserCacheRepositoryError {}

#[async_trait]
pub trait ISpaceUserCacheRepository: Send + Sync {
    async fn add_user(&self, user: &User) -> Result<(), SpaceUserCacheRepositoryError>;
    async fn find_user_by_id(&self, id: &CoachId) -> Result<User, SpaceUserCacheRepositoryError>;
    async fn find_all_users(&self) -> Result<Vec<User>, SpaceUserCacheRepositoryError>;
    async fn list_members_for_space(&self, space_id: &SpaceId) -> Result<Vec<User>, SpaceUserCacheRepositoryError>;
}