use crate::app::auth::domain::user::User;
use crate::app::auth::ports::{IUserRepository, RepositoryError};
use crate::app::auth::use_cases::perform_login::PerformLoginCommand;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use std::sync::Arc;

// `AuthUser` est un trait d'axum-login : son implémentation ne peut pas vivre
// dans `domain/`, que le CLAUDE.md veut sans dépendance framework. Elle est
// donc ici, dans la couche io du BC propriétaire du type — à quelques lignes
// de `AuthnBackend`, son unique consommateur.
impl axum_login::AuthUser for User {
    type Id = String;
    fn id(&self) -> Self::Id {
        self.id.to_string()
    }
    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

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
    type User = User;
    type Credentials = PerformLoginCommand;
    type Error = RepositoryError;

    async fn authenticate(
        &self,
        creds: PerformLoginCommand,
    ) -> Result<Option<User>, RepositoryError> {
        let Some(user) = self
            .user_repository
            .find_by_coach_name(&creds.coach_name)
            .await?
        else {
            return Ok(None);
        };
        let hash = PasswordHash::new(&user.password_hash)
            .map_err(|_| RepositoryError::Database("hash corrompu".into()))?;
        if Argon2::default()
            .verify_password(creds.password.expose().as_bytes(), &hash)
            .is_ok()
        {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<User>, RepositoryError> {
        self.user_repository.find_by_id(user_id).await
    }
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;
