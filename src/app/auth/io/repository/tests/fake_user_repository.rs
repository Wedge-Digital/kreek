use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::email::Email;
use crate::app::auth::domain::user::User;
use async_trait::async_trait;

pub enum FindResult {
    Found { password_hash: String },
    NotFound,
    DbError(String),
}
pub struct FakeUserRepository {
    pub find_result: FindResult,
}

#[async_trait]
impl IUserRepository for FakeUserRepository {
    async fn create(&self, _: &User) -> Result<(), RepositoryError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<User>, RepositoryError> {
        self.find_by_coach_name("").await
    }
    async fn find_by_legacy_id(&self, _: i32) -> Result<Option<User>, RepositoryError> {
        unimplemented!()
    }
    async fn update_password_hash(&self, _: &str, _: &str) -> Result<(), RepositoryError> {
        Ok(())
    }
    async fn find_by_coach_name(&self, _: &str) -> Result<Option<User>, RepositoryError> {
        match &self.find_result {
            FindResult::Found { password_hash } => Ok(Some(User::new(
                UserId::new(),
                CoachName::try_new("Bagouze").unwrap(),
                None,
                Email::try_new("coach@example.com").unwrap(),
                password_hash.clone(),
            ))),
            FindResult::NotFound => Ok(None),
            FindResult::DbError(msg) => Err(RepositoryError::Database(msg.clone())),
        }
    }
}
