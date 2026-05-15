use std::sync::Arc;
use sqlx::PgPool;
use crate::app::auth::io::repository::reset_token_repository::{IResetTokenRepository, ResetTokenRepository};
use crate::app::auth::io::repository::user_repository::UserRepository;
use crate::app::auth::ports::IUserRepository;

#[derive(Clone)]
pub struct AuthContext {
    pub user_repository:        Arc<dyn IUserRepository>,
    pub reset_token_repository: Arc<dyn IResetTokenRepository>,
}

impl AuthContext {
    pub fn new(pool: &PgPool) -> Self {
        Self {
            user_repository:        Arc::new(UserRepository::new(pool.clone())),
            reset_token_repository: Arc::new(ResetTokenRepository::new(pool.clone())),
        }
    }
}