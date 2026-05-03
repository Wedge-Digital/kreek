use async_trait::async_trait;
use crate::app::auth::domain::coach_name::CoachName;
use crate::app::auth::domain::email::Email;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::user::User;

pub enum FindResult { Found { password_hash: String }, NotFound, DbError(String) }
pub struct FakeUserRepository { pub find_result: FindResult }

#[async_trait]
impl IUserRepository for FakeUserRepository {
    async fn create(&self, _: &User) -> Result<(), RepositoryError> { unimplemented!() }
    async fn find_by_coach_name(&self, _: &str) -> Result<Option<User>, RepositoryError> {
        match &self.find_result {
            FindResult::Found { password_hash } => Ok(Some(User::new(
                UserId::new(),
                CoachName::new("Bagouze").unwrap(),
                Email::new("coach@example.com").unwrap(),
                password_hash.clone(),
            ))),
            FindResult::NotFound     => Ok(None),
            FindResult::DbError(msg) => Err(RepositoryError::Database(msg.clone())),
        }
    }
}
