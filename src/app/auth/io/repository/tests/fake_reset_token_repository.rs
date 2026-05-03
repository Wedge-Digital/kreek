use async_trait::async_trait;
use crate::app::auth::domain::coach_name::CoachName;
use crate::app::auth::domain::ResetToken::{ResetToken, Token};
use crate::app::auth::io::repository::reset_token_repository::IResetTokenRepository;
use crate::app::auth::ports::RepositoryError;

pub enum FindResult {
    Found,
    NotFound,
    DbError(String),
}

pub struct FakeResetTokenRepository {
    pub find_result: FindResult,
}

#[async_trait]
impl IResetTokenRepository for FakeResetTokenRepository {
    async fn find_by_token(&self, _: &str) -> Result<Option<ResetToken>, RepositoryError> {
        match &self.find_result {
            FindResult::Found => Ok(Some(ResetToken {
                token:      Token::new(),
                coach_name: CoachName::try_new("Bagouze").unwrap(),
                created_at: time::OffsetDateTime::now_utc(),
            })),
            FindResult::NotFound     => Ok(None),
            FindResult::DbError(msg) => Err(RepositoryError::Database(msg.clone())),
        }
    }

    async fn create(&self, _: &Token, _: &CoachName) -> Result<(), RepositoryError> {
        match &self.find_result {
            FindResult::DbError(msg) => Err(RepositoryError::Database(msg.clone())),
            _                        => Ok(()),
        }
    }

    async fn delete_by_token(&self, _: &str) -> Result<(), RepositoryError> {
        match &self.find_result {
            FindResult::DbError(msg) => Err(RepositoryError::Database(msg.clone())),
            _                        => Ok(()),
        }
    }
}