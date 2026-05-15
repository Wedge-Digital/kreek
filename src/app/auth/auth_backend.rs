use std::sync::Arc;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use crate::app::shared_kernel::email::Email;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::auth::use_cases::perform_login::PerformLoginCommand;
use crate::app::shared_kernel::coach_name::CoachName;
use crate::app::shared_kernel::common_types::UserId;
use crate::app::shared_kernel::user::User;

const BYPASS_USER_ID: &str = "00000000000000000000000000";

pub fn bypass_user() -> User {
    User::new(
        UserId::try_new(BYPASS_USER_ID).expect("BYPASS_USER_ID invalide"),
        CoachName::try_new("Dev User").expect("bypass coach_name invalide"),
        None,
        Email::try_new("dev@bypass.local").expect("bypass email invalide"),
        "bypass".to_string(),
    )
}

#[derive(Clone)]
pub struct AuthBackend {
    user_repository: Arc<dyn IUserRepository>,
    bypass_auth:     bool,
}

impl AuthBackend {
    pub fn new(user_repository: Arc<dyn IUserRepository>, bypass_auth: bool) -> Self {
        Self { user_repository, bypass_auth }
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
        if self.bypass_auth && user_id.as_str() == BYPASS_USER_ID {
            return Ok(Some(bypass_user()));
        }
        self.user_repository.find_by_id(user_id).await
    }
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;