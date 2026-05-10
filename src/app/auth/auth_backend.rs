use std::sync::Arc;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::auth::use_cases::perform_login::PerformLoginCommand;
use crate::app::shared_kernel::user::User;

#[derive(Clone)]
pub struct AuthBackend {
    user_repository: Arc<dyn IUserRepository>,
}

impl AuthBackend {
    pub fn new(user_repository: Arc<dyn IUserRepository>) -> Self {
        Self { user_repository }
    }
}

impl axum_login::AuthnBackend for AuthBackend {
    type User        = User;
    type Credentials = PerformLoginCommand;
    type Error       = RepositoryError;

    async fn authenticate(&self, creds: PerformLoginCommand) -> Result<Option<User>, RepositoryError> {
        let Some(user) = self.user_repository.find_by_coach_name(&creds.coach_name).await? else {
            return Ok(None);
        };
        let hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| RepositoryError::Database("hash corrompu".into()))?;
        if Argon2::default().verify_password(creds.password.as_bytes(), &hash).is_ok() {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    async fn get_user(&self, user_id: &axum_login::UserId<Self>) -> Result<Option<User>, RepositoryError> {
        self.user_repository.find_by_id(user_id).await
    }
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;